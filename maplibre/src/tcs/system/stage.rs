use crate::{
    context::MapContext,
    schedule::Stage,
    tcs::system::{IntoSystemContainer, SystemContainer},
};

#[derive(Default)]
pub struct SystemStage {
    systems: Vec<SystemContainer>,
}

impl SystemStage {
    #[must_use]
    pub fn with_system(mut self, system: impl IntoSystemContainer) -> Self {
        self.add_system(system);
        self
    }

    pub fn add_system(&mut self, system: impl IntoSystemContainer) -> &mut Self {
        self.systems.push(system.into_container());
        self
    }
}

impl Stage for SystemStage {
    fn run(&mut self, context: &mut MapContext) {
        for container in &mut self.systems {
            let name = container.system.name();
            #[cfg(feature = "trace")]
            let _span =
                tracing::info_span!("system", name = name.as_ref()).entered();
            // log::trace!("Running system: {}", name);
            container.system.run(context)
        }
    }
}
