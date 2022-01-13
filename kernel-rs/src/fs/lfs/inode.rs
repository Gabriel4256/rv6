use core::{iter::StepBy, mem, ops::Range};

use static_assertions::const_assert;
use zerocopy::{AsBytes, FromBytes};

use super::{FileName, Lfs, Path, NDIRECT, ROOTINO};
use crate::{
    arena::{Arena, ArrayArena},
    bio::BufData,
    fs::{lfs::superblock::IPB, Inode, InodeGuard, InodeType, Itable, RcInode, Tx},
    hal::hal,
    lock::SleepLock,
    param::NINODE,
    param::ROOTDEV,
    proc::KernelCtx,
    util::{memset, strong_pin::StrongPin},
};

/// Directory is a file containing a sequence of Dirent structures.
pub const DIRSIZ: usize = 14;

/// dirent size
pub const DIRENT_SIZE: usize = mem::size_of::<Dirent>();

#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(i16)]
pub enum DInodeType {
    None,
    Dir,
    File,
    Device,
}

pub struct InodeInner {
    /// inode has been read from disk?
    pub valid: bool,
    /// copy of disk inode
    pub typ: InodeType,
    pub nlink: i16,
    pub size: u32,
    pub addr_direct: [u32; NDIRECT],
    pub addr_indirect: u32,
}

/// On-disk inode structure
/// Both the kernel and user programs use this header file.
// It needs repr(C) because it's struct for in-disk representation
// which should follow C(=machine) representation
// https://github.com/kaist-cp/rv6/issues/52
#[repr(C)]
pub struct Dinode {
    /// File type
    pub typ: DInodeType,

    /// Major device number (T_DEVICE only)
    pub major: u16,

    /// Minor device number (T_DEVICE only)
    pub minor: u16,

    /// Number of links to inode in file system
    pub nlink: i16,

    /// Size of file (bytes)
    pub size: u32,

    /// Direct data block addresses
    pub addr_direct: [u32; NDIRECT],

    /// Indirect data block address
    pub addr_indirect: u32,
}

#[repr(C)]
#[derive(Default, AsBytes, FromBytes)]
pub struct Dirent {
    pub inum: u16,
    name: [u8; DIRSIZ],
}

impl Dirent {
    fn new(ip: &mut InodeGuard<'_, Lfs>, off: u32, ctx: &KernelCtx<'_, '_>) -> Result<Dirent, ()> {
        let mut dirent = Dirent::default();
        ip.read_kernel(&mut dirent, off, ctx)?;
        Ok(dirent)
    }

    /// Fill in name. If name is shorter than DIRSIZ, NUL character is appended as
    /// terminator.
    ///
    /// `name` must not contain NUL characters, but this is not a safety invariant.
    fn set_name(&mut self, name: &FileName<{ DIRSIZ }>) {
        let name = name.as_bytes();
        if name.len() == DIRSIZ {
            self.name.copy_from_slice(name);
        } else {
            self.name[..name.len()].copy_from_slice(name);
            self.name[name.len()] = 0;
        }
    }

    /// Returns slice which exactly contains `name`.
    ///
    /// It contains no NUL characters.
    fn get_name(&self) -> &FileName<{ DIRSIZ }> {
        let len = self.name.iter().position(|ch| *ch == 0).unwrap_or(DIRSIZ);
        // SAFETY: self.name[..len] doesn't contain '\0', and len must be <= DIRSIZ.
        unsafe { FileName::from_bytes(&self.name[..len]) }
    }
}

struct DirentIter<'id, 's, 't> {
    guard: &'s mut InodeGuard<'t, Lfs>,
    iter: StepBy<Range<u32>>,
    ctx: &'s KernelCtx<'id, 's>,
}

impl Iterator for DirentIter<'_, '_, '_> {
    type Item = (Dirent, u32);

    fn next(&mut self) -> Option<Self::Item> {
        let off = self.iter.next()?;
        let dirent = Dirent::new(self.guard, off, self.ctx).expect("DirentIter");
        Some((dirent, off))
    }
}

impl<'t> InodeGuard<'t, Lfs> {
    fn iter_dirents<'id, 's>(&'s mut self, ctx: &'s KernelCtx<'id, 's>) -> DirentIter<'id, 's, 't> {
        let iter = (0..self.deref_inner().size).step_by(DIRENT_SIZE);
        DirentIter {
            guard: self,
            iter,
            ctx,
        }
    }
}

// Directories
impl InodeGuard<'_, Lfs> {
    /// Write a new directory entry (name, inum) into the directory dp.
    pub fn dirlink(
        &mut self,
        name: &FileName<DIRSIZ>,
        inum: u32,
        tx: &Tx<'_, Lfs>,
        ctx: &KernelCtx<'_, '_>,
    ) -> Result<(), ()> {
        // Check that name is not present.
        if let Ok((ip, _)) = self.dirlookup(name, ctx) {
            ip.free((tx, ctx));
            return Err(());
        };

        // Look for an empty Dirent.
        let (mut de, off) = self
            .iter_dirents(ctx)
            .find(|(de, _)| de.inum == 0)
            .unwrap_or((Default::default(), self.deref_inner().size));
        de.inum = inum as _;
        de.set_name(name);
        self.write_kernel(&de, off, tx, ctx).expect("dirlink");
        Ok(())
    }

    /// Look for a directory entry in a directory.
    /// If found, return the entry and byte offset of entry.
    pub fn dirlookup(
        &mut self,
        name: &FileName<DIRSIZ>,
        ctx: &KernelCtx<'_, '_>,
    ) -> Result<(RcInode<Lfs>, u32), ()> {
        assert_eq!(self.deref_inner().typ, InodeType::Dir, "dirlookup not DIR");

        self.iter_dirents(ctx)
            .find(|(de, _)| de.inum != 0 && de.get_name() == name)
            .map(|(de, off)| {
                // TODO: replace the return type of fs() with Lfs
                todo!()
                // (
                //     ctx.kernel()
                //         .fs()
                //         .imap()
                //         .get_inode(self.dev, de.inum as u32),
                //     off,
                // )
            })
            .ok_or(())
    }
}

impl InodeGuard<'_, Lfs> {
    /// Copy a modified in-memory inode to disk.
    /// Must be called after every change to an ip->xxx field
    /// that lives on disk.
    pub fn update(&self, tx: &Tx<'_, Lfs>, ctx: &KernelCtx<'_, '_>) {
        let mut bp = hal()
            .disk()
            .read(self.dev, tx.fs.superblock().iblock(self.inum), ctx);

        const_assert!(IPB <= mem::size_of::<BufData>() / mem::size_of::<Dinode>());
        const_assert!(mem::align_of::<BufData>() % mem::align_of::<Dinode>() == 0);
        // SAFETY:
        // * dip is aligned properly.
        // * dip is inside bp.data.
        // * dip will not be read.
        let dip = unsafe {
            &mut *(bp.deref_inner_mut().data.as_mut_ptr() as *mut Dinode)
                .add(self.inum as usize % IPB)
        };

        let inner = self.deref_inner();
        match inner.typ {
            InodeType::Device { major, minor } => {
                dip.typ = DInodeType::Device;
                dip.major = major;
                dip.minor = minor;
            }
            InodeType::None => {
                dip.typ = DInodeType::None;
                dip.major = 0;
                dip.minor = 0;
            }
            InodeType::Dir => {
                dip.typ = DInodeType::Dir;
                dip.major = 0;
                dip.minor = 0;
            }
            InodeType::File => {
                dip.typ = DInodeType::File;
                dip.major = 0;
                dip.minor = 0;
            }
        }

        (*dip).nlink = inner.nlink;
        (*dip).size = inner.size;
        for (d, s) in (*dip).addr_direct.iter_mut().zip(&inner.addr_direct) {
            *d = *s;
        }
        (*dip).addr_indirect = inner.addr_indirect;

        // TODO: use transaction to write
        // tx.write(bp, ctx);
    }

    /// Inode content.
    /// If there is no such block, allocate one inode on disk.
    /// TODO: delete bmap
    pub fn disk_or_alloc(&mut self, bn: usize, tx: &Tx<'_, Lfs>, ctx: &KernelCtx<'_, '_>) -> u32 {
        self.disk_internal(bn, Some(tx), ctx)
    }

    pub fn disk(&mut self, bn: usize, ctx: &KernelCtx<'_, '_>) -> u32 {
        self.disk_internal(bn, None, ctx)
    }

    fn disk_internal(
        &mut self,
        bn: usize,
        tx_opt: Option<&Tx<'_, Lfs>>,
        ctx: &KernelCtx<'_, '_>,
    ) -> u32 {
        todo!()
    }

    /// Is the directory dp empty except for "." and ".." ?
    pub fn is_dir_empty(&mut self, ctx: &KernelCtx<'_, '_>) -> bool {
        let mut de: Dirent = Default::default();
        for off in (2 * DIRENT_SIZE as u32..self.deref_inner().size).step_by(DIRENT_SIZE) {
            self.read_kernel(&mut de, off, ctx)
                .expect("is_dir_empty: read_kernel");
            if de.inum != 0 {
                return false;
            }
        }
        true
    }
}

impl const Default for Inode<Lfs> {
    fn default() -> Self {
        Self::new()
    }
}

impl Inode<Lfs> {
    pub const fn new() -> Self {
        Self {
            dev: 0,
            inum: 0,
            inner: SleepLock::new(
                "inode",
                InodeInner {
                    valid: false,
                    typ: InodeType::None,
                    nlink: 0,
                    size: 0,
                    addr_direct: [0; NDIRECT],
                    addr_indirect: 0,
                },
            ),
        }
    }
}

impl Itable<Lfs> {
    pub const fn new_itable() -> Self {
        ArrayArena::<Inode<Lfs>, NINODE>::new("ITABLE")
    }

    /// Find the inode with number inum on device dev
    /// and return the in-memory copy. Does not lock
    /// the inode and does not read it from disk.
    pub fn get_inode(self: StrongPin<'_, Self>, dev: u32, inum: u32) -> RcInode<Lfs> {
        self.find_or_alloc(
            |inode| inode.dev == dev && inode.inum == inum,
            |inode| {
                inode.dev = dev;
                inode.inum = inum;
                inode.inner.get_mut().valid = false;
            },
        )
        .expect("[Itable::get_inode] no inodes")
    }

    /// Allocate an inode on device dev.
    /// Mark it as allocated by giving it type.
    /// Returns an unlocked but allocated and referenced inode.
    pub fn alloc_inode(
        self: StrongPin<'_, Self>,
        dev: u32,
        typ: InodeType,
        tx: &Tx<'_, Lfs>,
        ctx: &KernelCtx<'_, '_>,
    ) -> RcInode<Lfs> {
        for inum in 1..tx.fs.superblock().ninodes {
            let mut bp = hal().disk().read(dev, tx.fs.superblock().iblock(inum), ctx);

            const_assert!(IPB <= mem::size_of::<BufData>() / mem::size_of::<Dinode>());
            const_assert!(mem::align_of::<BufData>() % mem::align_of::<Dinode>() == 0);
            // SAFETY: dip is inside bp.data.
            let dip = unsafe {
                (bp.deref_inner_mut().data.as_mut_ptr() as *mut Dinode).add(inum as usize % IPB)
            };
            // SAFETY: i16 does not have internal structure.
            let t = unsafe { *(dip as *const i16) };
            // If t >= #(variants of DInodeType), UB will happen when we read dip.typ.
            assert!(t < core::mem::variant_count::<DInodeType>() as i16);
            // SAFETY: dip is aligned properly and t < #(variants of DInodeType).
            let dip = unsafe { &mut *dip };

            // a free inode
            if dip.typ == DInodeType::None {
                // SAFETY: DInode does not have any invariant.
                unsafe { memset(dip, 0u32) };
                match typ {
                    InodeType::None => dip.typ = DInodeType::None,
                    InodeType::Dir => dip.typ = DInodeType::Dir,
                    InodeType::File => dip.typ = DInodeType::File,
                    InodeType::Device { major, minor } => {
                        dip.typ = DInodeType::Device;
                        dip.major = major;
                        dip.minor = minor
                    }
                }

                // TODO: mark it allocated on the disk
                // tx.write(bp, ctx);

                // TODO: update Imap after the inode is allocated
                return self.get_inode(dev, inum);
            } else {
                bp.free(ctx);
            }
        }
        panic!("[Itable::alloc_inode] no inodes");
    }

    pub fn root(self: StrongPin<'_, Self>) -> RcInode<Lfs> {
        self.get_inode(ROOTDEV, ROOTINO)
    }

    pub fn namei(
        self: StrongPin<'_, Self>,
        path: &Path,
        tx: &Tx<'_, Lfs>,
        proc: &KernelCtx<'_, '_>,
    ) -> Result<RcInode<Lfs>, ()> {
        Ok(self.namex(path, false, tx, proc)?.0)
    }

    pub fn nameiparent<'s>(
        self: StrongPin<'_, Self>,
        path: &'s Path,
        tx: &Tx<'_, Lfs>,
        ctx: &KernelCtx<'_, '_>,
    ) -> Result<(RcInode<Lfs>, &'s FileName<{ DIRSIZ }>), ()> {
        let (ip, name_in_path) = self.namex(path, true, tx, ctx)?;
        let name_in_path = name_in_path.ok_or(())?;
        Ok((ip, name_in_path))
    }

    fn namex<'s>(
        self: StrongPin<'_, Self>,
        mut path: &'s Path,
        parent: bool,
        tx: &Tx<'_, Lfs>,
        ctx: &KernelCtx<'_, '_>,
    ) -> Result<(RcInode<Lfs>, Option<&'s FileName<{ DIRSIZ }>>), ()> {
        let mut ptr = if path.is_absolute() {
            self.root()
        } else {
            // TODO: replace the return type of proc with Lfs
            todo!()
            // ctx.proc().cwd().clone()
        };

        while let Some((new_path, name)) = path.skipelem() {
            path = new_path;

            let mut ip = ptr.lock(ctx);
            if ip.deref_inner().typ != InodeType::Dir {
                ip.free(ctx);
                ptr.free((tx, ctx));
                return Err(());
            }
            if parent && path.is_empty_string() {
                // Stop one level early.
                ip.free(ctx);
                return Ok((ptr, Some(name)));
            }
            let next = ip.dirlookup(name, ctx);
            ip.free(ctx);
            ptr.free((tx, ctx));
            ptr = next?.0
        }
        if parent {
            ptr.free((tx, ctx));
            return Err(());
        }
        Ok((ptr, None))
    }
}
