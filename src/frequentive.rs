use either::Either;
use futures::Future;
use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    select,
    sync::mpsc::{self, unbounded_channel},
    task::JoinHandle,
    time::sleep,
};

// #[async_trait]
// pub trait Cancellable<E> {
//     /// Sends shutdown signal and joins.
//     async fn cancel(self) -> Result<(), E>;
// }

// #[async_trait]
// pub trait Frequentive<A, E> {
//     // async fn frequent(&self, arg: A) -> Result<(), E>;
//     //
// }

struct DelayedResult<E> {
    jh: Option<JoinHandle<Result<(), E>>>,
    result: Option<Result<(), E>>,
}

impl <E: Send + Clone + 'static> DelayedResult<E> {
    fn delayed(jh: JoinHandle<Result<(), E>>) -> Self {
        Self {
            jh: Some(jh),
            result: None
        }
    }

    /// awaits joinhandle if it hasn't concluded yet.
    /// If joinhandle has concluded, returns the result on all subsequent calls.
    /// 
    /// If joinhandle panicked, then also panicks.
    async fn finish(&mut self) -> Result<(), E> {
        if let Some(jh) = self.jh.take() {
            let result = jh.await.unwrap();
            self.result = Some(result.clone()); // save for the future
            result
        } else if let Some(result) = self.result.clone() {
            result
        } else {
            panic!("impossible")
        }
    }
}

pub struct Periodic<E: Send + 'static> {
    /// for sending the shutdown signal
    shutdown_tx: mpsc::UnboundedSender<()>,
    state: Arc<Mutex<DelayedResult<E>>>,
}

impl<E> Periodic<E>
where
    E: Send + Clone + 'static,
{
    /// Spawns a new tokio task, calling `f` every `period`, but also stops when the returned
    /// `Periodic` is `cancel()`ed, by sending a shutdown message to the tokio task.
    pub fn spawn<Fut, F>(period: Duration, f: F) -> Self
    where
        Fut: Future<Output = Result<(), E>> + Send,
        F: Fn() -> Fut + Send + Sync + 'static,
    {
        let (tx, mut rx) = unbounded_channel();

        let jh: JoinHandle<Result<(), E>> = tokio::spawn(async move {
            loop {
                select! {
                    shutdown = rx.recv() => break Ok(()),
                    periodic = sleep(period) => {
                        f().await?
                    }
                };
            }
        });

        Self {
            shutdown_tx: tx,
            state: Arc::new(Mutex::new(DelayedResult::delayed(jh))),
        }
    }
}

impl<E: Send + 'static> Drop for Periodic<E> {
    fn drop(&mut self) {
        if self.state.lock().unwrap().jh.is_some() {
            panic!("Attempt to drop `Periodic` without calling `cancel()` on it would leak a background task which runs forever. This is a bug.");
        }
    }
}

impl<E: Send + Clone + 'static> Periodic<E> {
    async fn cancel(self) -> Result<(), E> {
        let mut lock = self.state.lock().unwrap();
        if let Some(result) = &*lock.result {
            result.clone()
        } else {
            
        }


        // // if no result has been deposited yet (= task hasn't finished yet), destroy it.
        // if let Some(result) = lock.take() {
        //     result
        // } else {
        //     drop(lock); // won't hurt, right (since we require `self` and not `&self`)? Shouldn't matter either way.
        //     if let Err(err) = self.tx.send(()) {
        //         // can only fail when receivers dead, in that case: ignore.
        //     }
        //     self.jh
        //         .await
        //         .expect("Failed to join(=cancel) periodic task: It appears to have panicked")
        // }
    }
}

// impl<E: Send + 'static> Drop for Periodic<E> {
//     fn drop(&mut self) {
//         let mut lock = self.result.lock().unwrap();
//         if !lock.is_some() {
//             if let Err(err) = self.shutdown_tx.send(()) {
//                 // can only fail when receivers dead, in that case: ignore.
//             }
//             // let joinresult = self.jh.aw
//         }
//         todo!()
//     }
// }
