
pub trait Provides<T> {

}

pub trait Combines<T, U> {
    fn get() -> impl T + U;
}

//////////
pub trait Rounds {

}

pub trait Player {

}

pub trait PlayerInRound {

}

impl <T: Rounds + Player> T {

}

// Any type T which 
impl <R: Rounds, P: Player, T: Combines<R, P>> T {

}

fn my_cmd<R: Rounds, P: Player>(arg: ) {

}

fn derp() {
    let h2 = hlist![1, false, 42f32];
    let folded = h2.foldl(hlist![
        |i, acc| 
    ], init)
}

///////////////////////////


pub struct Usage<N: Node> {
    _ph: PhantomData<N>
}
impl <N: Node> Usage<N> {
    pub fn with<F: Fn(&mut N::Ctx)>(&mut self, f: F) {
        todo!()
    }
}

pub trait Context {
    fn uses<'ctx, N: Node>(&'ctx mut self) -> &'ctx mut Usage<N>;
}

pub trait Node {
    type Ctx : Context;

    fn define(ctx: &mut Self::Ctx) -> Self
    where
        Self: Sized;
}

pub struct BattleFox<M: Node> {
    bf4: Arc<Bf4Client>,
    // extensions: Vec<Box<dyn Extension>>,
    main: M,
}

pub struct BattleFoxCtx {

}

impl Context for BattleFoxCtx {
    #[must_use]
    fn uses<'ctx, N: Node>(&'ctx mut self) -> &'ctx mut Usage<N> {
        todo!()
    }
}

impl <T: Node<Ctx = BattleFoxCtx>> BattleFox<T> {
    pub async fn run(bf4: Arc<Bf4Client>) -> Self {
        let mut root = BattleFoxCtx {
            // uses: Vec::new(),
        };
        let main = T::define(&mut root);
        Self {
            bf4,
            // extensions: Vec::new(),
            main,
        }
    }

    // pub fn add_ext<T: Extension + 'static>(&mut self) {
    //     let mut scope = ExtUpImpl {
    //         // cmds: Vec::new(),
    //     };
    //     let ext = T::up(&mut scope);
    //     self.extensions.push(Box::new(ext));
    // }

    // pub async fn run<T: Extension>(&mut self) {
    //     let mut events = self.bf4.event_stream();
    //     while let Some(event) = events.next().await {

    //     }
    // }
}

//////////////

pub trait Setupable {
    type SetterUpper;
}

struct Main;
impl Node for Main {
    type Ctx = BattleFoxCtx;

    fn define(ctx: &mut BattleFoxCtx) -> Self
    where
        Self: Sized
    {
        ctx.uses::<Rounds>().with(|rounds: &mut RoundsCtx| {
            rounds.uses::<SimpleCommands>();
        });


        // root.uses::<Rounds>(|rounds: &mut RootScope<Rounds>| {
        //     rounds.each::<Mapvote>(|mapvote: &mut RoundScope<MapVote>| {
        //         mapvote.
        //     });
        // });

        Self
    }

    fn modify(&mut self, )
}

