//! Queues [PhaseItems](crate::render::render_phase::PhaseItem) for rendering.
use crate::{
    context::MapContext,
    render::{
        eventually::{Eventually, Eventually::Initialized},
        render_commands::DrawMasks,
        render_phase::{DrawState, LayerItem, RenderPhase, TileMaskItem},
        tile_view_pattern::WgpuTileViewPattern,
    },
    tcs::tiles::Tile,
    vector::{render_commands::DrawVectorTiles, VectorBufferPool},
};

pub fn queue_system(MapContext { world, .. }: &mut MapContext) {
    let Some((
        Initialized(tile_view_pattern),
        Initialized(buffer_pool),
        mask_phase,
        layer_item_phase,
    )) = world.resources.query_mut::<(
        &mut Eventually<WgpuTileViewPattern>,
        &mut Eventually<VectorBufferPool>,
        &mut RenderPhase<TileMaskItem>,
        &mut RenderPhase<LayerItem>,
    )>()
    else {
        return;
    };

    let buffer_pool_index = buffer_pool.index();
    let mut queued_count = 0;

    for view_tile in tile_view_pattern.iter() {
        let coords = &view_tile.coords();
        tracing::trace!("Drawing tile at {coords}");

        // draw tile normal or the source e.g. parent or children
        view_tile.render(|source_shape| {
            // Draw masks for all source_shapes
            mask_phase.add(TileMaskItem {
                draw_function: Box::new(DrawState::<TileMaskItem, DrawMasks>::new()),
                source_shape: source_shape.clone(),
            });

            if let Some(layer_entries) = buffer_pool_index.get_layers(source_shape.coords()) {
                log::debug!("Found {} layer entries for tile {}", layer_entries.len(), source_shape.coords());
                for layer_entry in layer_entries {
                    // Draw tile
                    layer_item_phase.add(LayerItem {
                        draw_function: Box::new(DrawState::<LayerItem, DrawVectorTiles>::new()),
                        index: layer_entry.style_layer.index,
                        style_layer: layer_entry.style_layer.id.clone(),
                        tile: Tile {
                            coords: layer_entry.coords,
                        },
                        source_shape: source_shape.clone(),
                    });
                    queued_count += 1;
                }
            } else {
                // log::debug!("No layer entries found for tile {}", source_shape.coords());
            };
        });
    }
    if queued_count > 0 {
        log::info!("Queued {queued_count} layer items for rendering");
    }
}
