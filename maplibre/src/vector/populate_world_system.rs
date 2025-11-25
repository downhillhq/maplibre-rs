use std::{borrow::Cow, marker::PhantomData, rc::Rc};

use crate::{
    context::MapContext,
    environment::Environment,
    io::apc::{AsyncProcedureCall, Message},
    kernel::Kernel,
    tcs::system::System,
    vector::{transferables::*, VectorLayerData, VectorLayersDataComponent},
};

pub struct PopulateWorldSystem<E: Environment, T> {
    kernel: Rc<Kernel<E>>,
    phantom_t: PhantomData<T>,
}

impl<E: Environment, T> PopulateWorldSystem<E, T> {
    pub fn new(kernel: &Rc<Kernel<E>>) -> Self {
        Self {
            kernel: kernel.clone(),
            phantom_t: Default::default(),
        }
    }
}

impl<E: Environment, T: VectorTransferables> System for PopulateWorldSystem<E, T> {
    fn name(&self) -> Cow<'static, str> {
        "populate_world_system".into()
    }

    fn run(&mut self, MapContext { world, .. }: &mut MapContext) {
        // log::info!("PopulateWorldSystem::run started"); // Commented out to reduce spam
        let mut message_count = 0;
        for message in self.kernel.apc().receive(|message| {
            let tag = message.tag();
            let matches = message.has_tag(T::TileTessellated::message_tag())
                || message.has_tag(T::LayerMissing::message_tag())
                || message.has_tag(T::LayerTessellated::message_tag())
                || message.has_tag(T::LayerIndexed::message_tag());
            
            if !matches {
                log::warn!("Filter rejecting message with tag: {:?}. Expected one of: {:?}, {:?}, {:?}, {:?}", 
                    tag, 
                    T::TileTessellated::message_tag(),
                    T::LayerMissing::message_tag(),
                    T::LayerTessellated::message_tag(),
                    T::LayerIndexed::message_tag()
                );
            }
            matches
        }) {
            message_count += 1;
            let message: Message = message;
            if message.has_tag(T::TileTessellated::message_tag()) {
                let message = message.into_transferable::<T::TileTessellated>();
                let coords = message.coords();
                let Some(component) = world
                    .tiles
                    .query_mut::<&mut VectorLayersDataComponent>(coords)
                else {
                    log::warn!("Received TileTessellated for {coords} but component not found");
                    continue;
                };

                component.done = true;
                log::info!("Marked tile {coords} as done, has {} layers", component.layers.len());
            } else if message.has_tag(T::LayerMissing::message_tag()) {
                let message = message.into_transferable::<T::LayerMissing>();
                let coords = message.coords();
                let layer_name = message.layer_name().to_string();
                let Some(component) = world
                    .tiles
                    .query_mut::<&mut VectorLayersDataComponent>(coords)
                else {
                    log::warn!("Received LayerMissing for {coords} but component not found");
                    continue;
                };

                component
                    .layers
                    .push(VectorLayerData::Missing(message.to_layer()));
                log::info!("Added missing layer {} for tile {}", layer_name, coords);
            } else if message.has_tag(T::LayerTessellated::message_tag()) {
                let message = message.into_transferable::<T::LayerTessellated>();
                let coords = message.coords();
                // FIXME: Handle points!
                /*if message.is_empty() {
                    continue;
                }*/

                let Some(component) = world
                    .tiles
                    .query_mut::<&mut VectorLayersDataComponent>(coords)
                else {
                    log::warn!("Received LayerTessellated for {coords} but component not found");
                    continue;
                };

                component
                    .layers
                    .push(VectorLayerData::Available(message.to_layer()));
                log::info!("Added tessellated layer for tile {coords}, total layers: {}", component.layers.len());
            } else if message.has_tag(T::LayerIndexed::message_tag()) {
                let message = message.into_transferable::<T::LayerIndexed>();
                world
                    .tiles
                    .geometry_index
                    .index_tile(&message.coords(), message.to_tile_index());
            }
        }
        if message_count > 0 {
            log::info!("PopulateWorldSystem processed {message_count} messages");
        }
    }
}
