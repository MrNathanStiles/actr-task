use std::{
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, SendError, Sender},
    },
    thread::{self, available_parallelism},
};
pub type Task = Box<dyn FnOnce() + Send + Sync>;

struct TaskManagerData<F> {
    next: usize,
    task_assignment: Vec<Sender<F>>,
    parallel: usize,
}
pub struct TaskManager<F>
where
    F: FnOnce(),
{
    data: Arc<Mutex<TaskManagerData<F>>>,
}

impl<F> TaskManager<F>
where
    F: 'static + FnOnce() + Sync + Send,
{
    pub fn new() -> Self {
        let parallel = available_parallelism().unwrap();
        let mut data = TaskManagerData {
            task_assignment: Vec::new(),
            next: 0,
            parallel: parallel.into(),
        };

        for _ in 0..data.parallel {
            let (task_assignment, task_list) = mpsc::channel::<F>();
            data.task_assignment.push(task_assignment);
            thread::spawn(move || TaskManager::thread_worker(task_list));
        }

        Self {
            data: Arc::new(Mutex::new(data)),
        }
    }

    fn thread_worker(task_list: Receiver<F>) {
        loop {
            match task_list.recv() {
                Ok(f) => f(),
                _ => return,
            }
        }
    }

    pub fn work(&self, f: F) -> Result<(), SendError<F>> {
        let mut data = self.data.lock().unwrap();
        let next = data.next;
        data.next += 1;
        let task_assignment = data.task_assignment[next % data.parallel].clone();
        drop(data);

        task_assignment.send(f)
    }
}

impl<F> Clone for TaskManager<F>
where
    F: 'static + FnOnce() + Sync + Send,
{
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
        }
    }
}
