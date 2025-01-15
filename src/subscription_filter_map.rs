use iced::Subscription;
use iced_futures::{
    subscription::{from_recipe, into_recipes},
    MaybeSend,
};

struct SubscriptionExt<T>(Subscription<T>);

impl<T> SubscriptionExt<T> {
    /// Transforms the [`Subscription`] output with the given function.
    ///
    /// # Panics
    /// The closure provided must be a non-capturing closure. The method
    /// will panic in debug mode otherwise.
    pub fn filter_map<F, A>(mut self, f: F) -> Subscription<A>
    where
        T: 'static,
        F: Fn(T) -> Option<A> + MaybeSend + Clone + 'static,
        A: 'static,
    {
        debug_assert!(
            std::mem::size_of::<F>() == 0,
            "the closure {} provided in `Subscription::map` is capturing",
            std::any::type_name::<F>(),
        );

        let recipes = into_recipes(self.0).drain(..)
            .filter_map(move |recipe| {
                let a = Box::new(Map::new(recipe, f.clone())) as Box<dyn Recipe<Output = A>>;
                Some
            })
            .collect();


        Subscription {
            recipes: recipes
                .drain(..)
                .filter_map(move |recipe| {
                    Box::new(Map::new(recipe, f.clone())) as Box<dyn Recipe<Output = A>>
                })
                .collect(),
        }

        from_recipe(recipes[0])

    }
}
