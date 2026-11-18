mod bucket;
mod list;

use wie_util::Result;

use crate::{
    ArmCore,
    core::{HEAP_BASE, HEAP_SIZE},
};

use self::{
    bucket::{BUCKET_MAX, BucketAllocator},
    list::ListAllocator,
};

pub struct Allocator;

impl Allocator {
    pub fn init(core: &mut ArmCore) -> Result<()> {
        // Commit heap pages only when an allocation reaches them.
        ListAllocator::init(core, HEAP_BASE, HEAP_SIZE / 2)?;
        BucketAllocator::init(core, HEAP_BASE + HEAP_SIZE / 2, HEAP_SIZE / 2)?;

        Ok(())
    }

    pub fn alloc(core: &mut ArmCore, size: u32) -> Result<u32> {
        if size > BUCKET_MAX as _ {
            ListAllocator::alloc(core, HEAP_BASE, HEAP_SIZE / 2, size)
        } else {
            BucketAllocator::alloc(core, HEAP_BASE + HEAP_SIZE / 2, size)
        }
    }

    pub fn free(core: &mut ArmCore, address: u32, size: u32) -> Result<()> {
        if size > BUCKET_MAX as _ {
            ListAllocator::free(core, address)
        } else {
            BucketAllocator::free(core, HEAP_BASE + HEAP_SIZE / 2, address, size)
        }
    }

    pub fn is_allocated(core: &ArmCore, address: u32, size: u32) -> Result<bool> {
        if size > BUCKET_MAX as _ {
            ListAllocator::is_allocated(core, HEAP_BASE, HEAP_SIZE / 2, address, size)
        } else {
            BucketAllocator::is_allocated(core, HEAP_BASE + HEAP_SIZE / 2, address, size)
        }
    }
}

#[cfg(test)]
mod tests {
    use wie_util::{Result, read_generic};

    use crate::{
        Allocator, ArmCore,
        core::{HEAP_BASE, HEAP_SIZE},
    };

    #[test]
    fn lazy_mapping_preserves_live_allocations_and_supports_reuse() -> Result<()> {
        let mut core = ArmCore::new(wie_backend::Options {
            enable_gdbserver: false,
            aot: None,
            profile: None,
        })?;
        Allocator::init(&mut core)?;
        assert!(!Allocator::is_allocated(&core, HEAP_BASE + 0x400000, 4096)?);

        let small = Allocator::alloc(&mut core, 12)?;
        core.write_bytes(small, &[0xa5; 12])?;
        let small_next = Allocator::alloc(&mut core, 12)?;
        core.write_bytes(small_next, &[0x5a; 12])?;
        let mut check = [0u8; 12];
        core.read_bytes(small, &mut check)?;
        assert_eq!(check, [0xa5; 12], "mapping a neighboring slot must not clear live data");

        let large = Allocator::alloc(&mut core, 131072)?;
        core.write_bytes(large, &[0x7c; 12])?;
        core.write_bytes(large + 131072 - 12, &[0x3e; 12])?;
        core.read_bytes(large + 131072 - 12, &mut check)?;
        assert_eq!(check, [0x3e; 12]);
        let large_next = Allocator::alloc(&mut core, 65536)?;
        core.read_bytes(large, &mut check)?;
        assert_eq!(check, [0x7c; 12]);
        Allocator::free(&mut core, large, 131072)?;
        Allocator::free(&mut core, large_next, 65536)?;
        let merged = Allocator::alloc(&mut core, 180000)?;
        assert_eq!(merged, large, "adjacent freed list blocks should remain reusable");
        core.write_bytes(merged + 180000 - 12, &[0x31; 12])?;
        core.read_bytes(merged + 180000 - 12, &mut check)?;
        assert_eq!(check, [0x31; 12]);
        Allocator::free(&mut core, merged, 180000)?;
        Allocator::free(&mut core, small, 12)?;
        Allocator::free(&mut core, small_next, 12)?;
        Ok(())
    }

    #[test]
    fn allocation_status_tracks_bucket_and_list_allocations() -> Result<()> {
        let mut core = ArmCore::new(wie_backend::Options {
            enable_gdbserver: false,
            aot: None,
            profile: None,
        })?;
        Allocator::init(&mut core)?;
        assert!(
            read_generic::<u32, _>(&core, HEAP_BASE + HEAP_SIZE / 4).is_err(),
            "untouched heap stays uncommitted"
        );

        let bucket = Allocator::alloc(&mut core, 12)?;
        let list = Allocator::alloc(&mut core, 1024)?;
        assert!(Allocator::is_allocated(&core, bucket, 12)?);
        assert!(Allocator::is_allocated(&core, list, 1024)?);
        assert!(!Allocator::is_allocated(&core, bucket + 4, 12)?);

        Allocator::free(&mut core, bucket, 12)?;
        Allocator::free(&mut core, list, 1024)?;
        assert!(!Allocator::is_allocated(&core, bucket, 12)?);
        assert!(!Allocator::is_allocated(&core, list, 1024)?);

        Ok(())
    }
}
