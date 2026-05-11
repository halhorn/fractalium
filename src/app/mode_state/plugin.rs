//! [`AppScreen`](super::routing::AppScreen) の登録と、起動時の [`NextState`](bevy::prelude::NextState) 注入。

use bevy::prelude::*;

use super::routing::AppScreen;
use super::startup::{
    resolve_initial_app_screen, BootstrapUsedPresetEnv, ShareUrlRestoredFractal,
};

/// `AppScreen` を [`States`] として載せ、[Startup](bevy::prelude::Startup) で初期遷移を確定する。
pub struct AppScreenPlugin;

impl Plugin for AppScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BootstrapUsedPresetEnv>()
            .init_resource::<ShareUrlRestoredFractal>()
            .init_state::<AppScreen>();

        #[cfg(target_arch = "wasm32")]
        {
            app.add_systems(
                Startup,
                resolve_initial_app_screen.after(crate::app::share::sync::hydrate_from_url),
            );
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            app.add_systems(Startup, resolve_initial_app_screen);
        }
    }
}
