//! Implementation of [`TaskManager`]
//!
//! It is only used to manage processes and schedule process based on ready queue.
//! Other CPU process monitoring functions are in Processor.


use super::{TaskControlBlock,current_task,TaskInfoBlock};
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;

use crate::config::{PAGE_SIZE,BigStride};
use crate::mm::{PhysAddr,VirtPageNum,VirtAddr,MemorySet,MapPermission};

pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

// YOUR JOB: FIFO->Stride
/// A simple FIFO scheduler.
impl TaskManager {
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        let mut maxpass=0x7fffffff;
        let mut mini=0;
        for i in 0..self.ready_queue.len()
        {
            if self.ready_queue[i].get_task_pass()<maxpass{
                maxpass=self.ready_queue[i].get_task_pass();
                mini=i;
            }
        }
        let mut inner=self.ready_queue[mini].inner_exclusive_access();
        inner.pass+=inner.stride;
        drop(inner);
        self.ready_queue.remove(mini)
    }
    fn get_current_task_info(&self) -> TaskInfoBlock
    {
        let taskctrblock=current_task().unwrap();
        let mut tskinner = taskctrblock.inner_exclusive_access();
        
        //it's a vec not an array
        tskinner.get_task_info()
    }
    fn change_syscall_time(&self,syscall_id:usize)
    {
        let taskctrblock=current_task().unwrap();
        let mut inner = taskctrblock.inner_exclusive_access();
        inner.taskinfo.task_syscall_times[syscall_id]+=1;
    }
    fn task_map_an_area(&self,_start: usize, _len: usize, _port: usize) -> isize
    {
        let taskctrblock=current_task().unwrap();
        let mut inner = taskctrblock.inner_exclusive_access();

        let mut page_num=_len/PAGE_SIZE;
        if _len%PAGE_SIZE!=0{
            page_num+=1;
        }
        for i in 0..page_num{
            let vir_page_start=VirtPageNum(_start/PAGE_SIZE+i);
            if inner.memory_set.have_mapped(vir_page_start)
            {
                return -1;
            }
        }
        let permission=MapPermission::from_bits(((_port<<1)|16)as u8);
        inner.memory_set.insert_framed_area(VirtAddr(_start) ,VirtAddr(_start+_len),permission.unwrap());
        0
    }
    fn task_unmap_an_area(&self,_start: usize, _len: usize) -> isize
    {
        let taskctrblock=current_task().unwrap();
        let mut inner = taskctrblock.inner_exclusive_access();
        
        let mut page_num=_len/PAGE_SIZE;
        if _len%PAGE_SIZE!=0{
            page_num+=1;
        }
        for i in 0..page_num{
            let vir_page_start=VirtPageNum(_start/PAGE_SIZE+i);
            if !inner.memory_set.have_mapped(vir_page_start)
            {
                return -1;
            }
            inner.memory_set.ummap_area(vir_page_start);
        }
        0
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().add(task);
}

pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch()
}

pub fn get_current_task() -> TaskInfoBlock
{
    TASK_MANAGER.exclusive_access().get_current_task_info()
}
pub fn change_current_syscall_time(syscall_id:usize)
{
    TASK_MANAGER.exclusive_access().change_syscall_time(syscall_id);
}
pub fn task_map_an_area(_start: usize, _len: usize, _port: usize) -> isize
{
    TASK_MANAGER.exclusive_access().task_map_an_area(_start, _len, _port)
}
pub fn task_unmap_an_area(_start: usize, _len: usize) -> isize
{
    TASK_MANAGER.exclusive_access().task_unmap_an_area(_start,_len)
}