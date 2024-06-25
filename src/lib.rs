// SPDX-License-Identifier: GPL-2.0
//! Rust reset-bsta1000b

#![no_std]
#![feature(allocator_api)]

use kernel::{
    bindings,
    delay::coarse_sleep,
    device,
    error,
    module_platform_driver,  
    of,
    platform,  
    prelude::*,
    reset::{self, ResetRegistration},
    sync::{Arc,ArcBorrow},
};

use core::{
    ops::DerefMut,
    time::Duration,
};

use bst_reset_rust::{
    BstResetManager,
    RstResId,
    RST_HOLD_TIME,
    RESET_LONG_HOLD_TIME,
    ZERO_ASSERT_ONE_DEASSERT,
};

module_platform_driver! {
    type: BstResetDriver,
    name: "reset_bsta1000b",
    license: "GPL v2",
    initcall: "arch",
}

// Define the device ID table for module matching
kernel::module_of_id_table!(BST_RESET_MOD_TABLE, BST_RESET_OF_MATCH_TABLE);
// Define the ID array for device tree matching
kernel::define_of_id_table! {BST_RESET_OF_MATCH_TABLE, (), [
    (of::DeviceId::Compatible(b"bst,a1000b-rstc"), None),
]}

// Define the main driver structure
struct BstResetDriver;

// SAFETY: `BstMap` holds a non-null pointer to GPIO registers, which is safe to be used from any thread.
unsafe impl Send for BstMap {}
// SAFETY: `BstMap` holds a non-null pointer to GPIO registers, references to which are safe to be used from any thread.
unsafe impl Sync for BstMap {}

// Define a structure to hold reset addresses
#[derive(Clone)]
struct BstMap{
    bst_address:[Option<*mut u8>; 5]
}

// Type definitions for reset registrations and device data
type ResetRegistrations = reset::ResetRegistration<BstResetDriver>;
type ResetDeviceData = device::Data<ResetRegistrations, (), BstMap>;

// Implement the platform driver for `BstResetDriver`
impl platform::Driver for BstResetDriver {
    // Use the ID table for driver matching
    kernel::driver_of_id_table!(BST_RESET_OF_MATCH_TABLE);
    type Data = Arc<ResetDeviceData>;
    
    // Probe function to initialize the driver
    fn probe(pdev: &mut platform::Device, _id_info: Option<&Self::IdInfo>) -> Result<Self::Data> {
        dev_info!(pdev, "{} driver in Rust (probe)\n", pdev.name());

        const TOTAL_REGISTERS: usize = 5;
        let mut a1000b_rst_addr: [Option<*mut u8>; TOTAL_REGISTERS] = [None; TOTAL_REGISTERS];
        
        // Map register resources
        for i in (RstResId::TOP_CRM_BLOCK_SW_RST0 as usize)..=(RstResId::LSP1_RST_CTRL_REG as usize) {
            let i_u32: u32 = i.try_into().unwrap();
            let reg_base:*mut u8 = pdev.ioremap_resource(i_u32)?;
            a1000b_rst_addr[i] = Some(reg_base);
            if a1000b_rst_addr[i].is_none() {
                pr_err!("Could not remap register memory for register {}\n", i);
                return Err(error::code::ENOMEM);
            }
        }
        let reg_data = BstMap {bst_address:a1000b_rst_addr};

        // Register Reset                  
        let resetdata = kernel::new_device_data!(
            ResetRegistration::<BstResetDriver>::new(),
            (),
            reg_data,
            "reset Registrations"
        )?;
        
        let arc_resetdata:Arc<ResetDeviceData> = Arc::<ResetDeviceData>::from(resetdata);
        
        kernel::reset_controller_register!(
            unsafe {Pin::new_unchecked(arc_resetdata.registrations().ok_or(ENXIO)?.deref_mut()) },
            pdev,
            50,
            arc_resetdata.clone(),
        )?;
        Ok(arc_resetdata)
    }
}

// Implement the `Drop` trait for the driver
impl Drop for BstResetDriver {
    fn drop(&mut self) {
        pr_info!("BST-RESET driver in Rust (exit)\n");
    }
}

// Implement the reset operations for the driver
#[vtable]
impl reset::ResetDriverOps for BstResetDriver {
    type Data = Arc<ResetDeviceData>;

    // Assert the reset signal
    fn assert(data: ArcBorrow<'_, ResetDeviceData>, rst_id: u64) -> Result<i32> {
        let bstops_address = data.bst_address;
        let rst_id_usize = rst_id as usize;
        let manager = BstResetManager::new(bstops_address);
        
        if let Some(bst_rst_map) = &manager.bsta1000b_map[rst_id_usize] {
            let reg_val = readl(bst_rst_map.addr as usize);
            let new_val = if bst_rst_map.flags & ZERO_ASSERT_ONE_DEASSERT != 0 {
                reg_val & !(1 << bst_rst_map.bit_idx)
            } else {
                reg_val | (1 << bst_rst_map.bit_idx)
            };
            writel(new_val, bst_rst_map.addr as usize);
            return Ok(0);
        } else {
            pr_err!("Invalid reset ID: {}\n", rst_id_usize);
            return Err(error::code::EINVAL);
        }
    }

    // Deassert the reset signal
    fn deassert(data: ArcBorrow<'_, ResetDeviceData>, rst_id: u64) -> Result<i32> {
        let bstops_address = data.bst_address;
        let rst_id_usize = rst_id as usize;
        let manager = BstResetManager::new(bstops_address);
        
        if let Some(bst_rst_map) = &manager.bsta1000b_map[rst_id_usize] {
            let reg_val = readl(bst_rst_map.addr as usize);
            let new_val = if bst_rst_map.flags & ZERO_ASSERT_ONE_DEASSERT != 0 {
                reg_val | (1 << bst_rst_map.bit_idx)
            } else {
                reg_val & !(1 << bst_rst_map.bit_idx)
            };
            writel(new_val, bst_rst_map.addr as usize);
            return Ok(0);
        } else {
            pr_err!("Invalid reset ID: {}\n", rst_id_usize);
            return Err(error::code::EINVAL);
        }
    }

    // Check the reset status
    fn status(data: ArcBorrow<'_, ResetDeviceData>, rst_id: u64) -> Result<i32> {
        let bstops_address = data.bst_address;
        let rst_id_usize = rst_id as usize;
        let manager = BstResetManager::new(bstops_address);
        
        if let Some(bst_rst_map) = &manager.bsta1000b_map[rst_id_usize] {
            let reg_val = readl(bst_rst_map.addr as usize);
            let status = if bst_rst_map.flags & ZERO_ASSERT_ONE_DEASSERT != 0 {
                !(reg_val & (1 << bst_rst_map.bit_idx)) as i32
            } else {
                !(!(reg_val & (1 << bst_rst_map.bit_idx))) as i32
            };
            return Ok(status);
        } else {
            pr_err!("Invalid reset ID: {}\n", rst_id_usize);
            return Err(error::code::EINVAL);
        }
    }

    // Perform a reset operation
    fn reset(data: ArcBorrow<'_, ResetDeviceData>, rst_id: u64) -> Result<i32> {
        let bstops_address = data.bst_address;
        let rst_id_usize = rst_id as usize;
        let manager = BstResetManager::new(bstops_address);
        
        if let Some(bst_rst_map) = &manager.bsta1000b_map[rst_id_usize] {
            BstResetDriver::assert(data, rst_id)?;
            coarse_sleep(Duration::from_millis(RST_HOLD_TIME));
            if bst_rst_map.flags & RESET_LONG_HOLD_TIME != 0 {
                coarse_sleep(Duration::from_millis(RST_HOLD_TIME));
            }
            BstResetDriver::deassert(data, rst_id)?;
            coarse_sleep(Duration::from_millis(RST_HOLD_TIME));
            if bst_rst_map.flags & RESET_LONG_HOLD_TIME != 0 {
                coarse_sleep(Duration::from_millis(RST_HOLD_TIME));
            }
            return Ok(0);
        } else {
            pr_err!("Invalid reset ID: {}\n", rst_id_usize);
            return Err(error::code::EINVAL);
        }
    }
}

// Function to read a 32-bit value from a memory-mapped register
fn readl(addr: usize) -> u32 {
    let val = unsafe { bindings::readl(addr as _) };
    val
}
    
// Function to write a 32-bit value to a memory-mapped register
fn writel(val: u32, addr: usize) {
    unsafe { bindings::writel(val, addr as _) }
}
    