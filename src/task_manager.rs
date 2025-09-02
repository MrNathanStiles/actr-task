use std::{
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, SendError, Sender},
    },
    thread::{self, available_parallelism},
};
pub type Task = dyn FnOnce() + Send + 'static;
pub type TaskBox = Box<Task>;

struct TaskManagerData {
    next: usize,
    task_assignment: Vec<Sender<TaskBox>>,
    parallel: usize,
}
pub struct TaskManager {
    data: Arc<Mutex<TaskManagerData>>,
}

impl TaskManager {
    pub fn new() -> Self {
        let parallel = available_parallelism().unwrap();
        let mut data = TaskManagerData {
            task_assignment: Vec::new(),
            next: 0,
            parallel: parallel.into(),
        };

        for _ in 0..data.parallel {
            let (task_assignment, task_list) = mpsc::channel::<TaskBox>();
            data.task_assignment.push(task_assignment);
            thread::spawn(move || TaskManager::thread_worker(task_list));
        }

        Self {
            data: Arc::new(Mutex::new(data)),
        }
    }

    fn thread_worker(task_list: Receiver<TaskBox>) {
        loop {
            match task_list.recv() {
                Ok(f) => f(),
                _ => return,
            }
        }
    }

    pub fn work(&self, f: impl FnOnce() + Send + 'static) -> Result<(), SendError<TaskBox>> {
        let mut data = self.data.lock().unwrap();
        let next = data.next;
        data.next += 1;
        let task_assignment = data.task_assignment[next % data.parallel].clone();
        drop(data);

        task_assignment.send(Box::new(f))
    }
}

impl Clone for TaskManager {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
        }
    }
}
