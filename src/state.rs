use tokio::sync::{RwLock, OnceCell, Notify};
use std::sync::Arc;
use crate::audio_source::{AudioSource, WaveSource};
pub struct AudioState {
    source: Arc<RwLock<Box<dyn AudioSource>>>,
    volume: Arc<RwLock<f32>>,
    paused: Arc<RwLock<bool>>,
    volume_notify: Arc<RwLock<Option<Arc<Notify>>>>,
    pause_notify: Arc<RwLock<Option<Arc<Notify>>>>,
}
impl AudioState {
    fn new() -> Self {
        Self {
            source: Arc::new(RwLock::new(Box::new(WaveSource::default()))),
            volume: Arc::new(RwLock::new(1.0)),
            paused: Arc::new(RwLock::new(false)),
            volume_notify: Arc::new(RwLock::new(None)),
            pause_notify: Arc::new(RwLock::new(None)),
        }
    }
    pub fn get_source(&self) -> Arc<RwLock<Box<dyn AudioSource>>> {
        Arc::clone(&self.source)
    }
    pub async fn set_source(&self, new_source: Box<dyn AudioSource>) {
        let mut source = self.source.write().await;
        *source = new_source;
    }
    pub async fn get_volume(&self) -> f32 {
        *self.volume.read().await
    }
    pub async fn set_volume(&self, vol: f32) {
        let clamped = vol.clamp(0.0, 1.0);
        let mut volume = self.volume.write().await;
        *volume = clamped;
        drop(volume);
        if let Some(notify) = self.volume_notify.read().await.as_ref() {
            notify.notify_one();
        }
    }
    pub async fn is_muted(&self) -> bool {
        *self.paused.read().await
    }
    pub async fn set_muted(&self, paused: bool) {
        let mut p = self.paused.write().await;
        *p = paused;
        drop(p);
        if let Some(notify) = self.pause_notify.read().await.as_ref() {
            notify.notify_one();
        }
    }
    pub async fn toggle_muted(&self) -> bool {
        let mut p = self.paused.write().await;
        *p = !*p;
        let new_state = *p;
        drop(p);
        if let Some(notify) = self.pause_notify.read().await.as_ref() {
            notify.notify_one();
        }
        new_state
    }
    pub async fn set_volume_notify(&self, notify: Arc<Notify>) {
        let mut vn = self.volume_notify.write().await;
        *vn = Some(notify);
    }
    pub async fn set_muted_notify(&self, notify: Arc<Notify>) {
        let mut pn = self.pause_notify.write().await;
        *pn = Some(notify);
    }
}
static AUDIO_STATE: OnceCell<AudioState> = OnceCell::const_new();
async fn get_audio_state() -> &'static AudioState {
    AUDIO_STATE.get_or_init(|| async { AudioState::new() }).await
}
pub async fn get_source() -> Arc<RwLock<Box<dyn AudioSource>>> {
    get_audio_state().await.get_source()
}
pub async fn set_source(source: Box<dyn AudioSource>) {
    get_audio_state().await.set_source(source).await;
}
pub async fn get_volume() -> f32 {
    get_audio_state().await.get_volume().await
}
pub async fn set_volume(volume: f32) {
    get_audio_state().await.set_volume(volume).await;
}
pub async fn is_muted() -> bool {
    get_audio_state().await.is_muted().await
}
pub async fn set_muted(paused: bool) {
    get_audio_state().await.set_muted(paused).await;
}
pub async fn toggle_mute() -> bool {
    get_audio_state().await.toggle_muted().await
}
pub async fn set_volume_notify(notify: Arc<Notify>) {
    get_audio_state().await.set_volume_notify(notify).await;
}
pub async fn set_mute_notify(notify: Arc<Notify>) {
    get_audio_state().await.set_muted_notify(notify).await;
}
