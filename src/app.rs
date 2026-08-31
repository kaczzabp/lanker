use crate::config::Config;
use crate::network::{self, DiscoveredDevice};
use crate::transfer::{self, IncomingRequest, PendingFile, TransferState};
use crate::ui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(PartialEq, Clone, Copy)]
pub enum View {
    Files,
    Devices,
    Transfers,
    Log,
}

pub struct LankerApp {
    pub config: Config,
    pub devices: Arc<Mutex<Vec<DiscoveredDevice>>>,
    pub transfer_state: Arc<Mutex<TransferState>>,
    pub pending_files: Vec<PendingFile>,
    pub selected_device: Option<usize>,
    pub show_settings: bool,
    pub send_cancel: Arc<AtomicBool>,
    pub receive_cancel: Arc<AtomicBool>,
    pub local_ip: String,
    pub active_view: View,
    /// Currently pending incoming transfer request (shown as accept/decline dialog).
    pub incoming_request: Arc<Mutex<Option<IncomingRequest>>>,
    /// Gate allowing a single active transfer (send or receive) at a time,
    /// so the shared TransferState never gets corrupted by overlapping transfers.
    active_transfer: Arc<AtomicBool>,
    tokio_handle: tokio::runtime::Handle,
}

impl LankerApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        config: Config,
        tokio_handle: tokio::runtime::Handle,
    ) -> Self {
        let devices: Arc<Mutex<Vec<DiscoveredDevice>>> = Arc::new(Mutex::new(Vec::new()));
        let transfer_state = Arc::new(Mutex::new(TransferState::default()));
        let send_cancel = Arc::new(AtomicBool::new(false));
        let receive_cancel = Arc::new(AtomicBool::new(false));
        let local_ip = network::get_local_ip();
        let incoming_request: Arc<Mutex<Option<IncomingRequest>>> = Arc::new(Mutex::new(None));
        let active_transfer = Arc::new(AtomicBool::new(false));
        ui::apply_theme(&cc.egui_ctx);

        network::spawn_broadcast(
            &tokio_handle,
            config.device_name.clone(),
            config.port,
            config.discovery_port,
            cc.egui_ctx.clone(),
        );

        network::spawn_listener(
            &tokio_handle,
            config.discovery_port,
            devices.clone(),
            cc.egui_ctx.clone(),
        );

        transfer::spawn_receiver(
            &tokio_handle,
            config.port,
            config.download_dir.clone(),
            config.chunk_size,
            transfer_state.clone(),
            receive_cancel.clone(),
            incoming_request.clone(),
            active_transfer.clone(),
            cc.egui_ctx.clone(),
        );

        tracing::info!(
            "Lanker started: {} at {}:{}", 
            config.device_name, local_ip, config.port
        );

        {
            let mut st = transfer_state.lock().unwrap();
            st.add_log(format!("Lanker started as '{}' on {}:{}", config.device_name, local_ip, config.port));
        }

        Self {
            config,
            devices,
            transfer_state,
            pending_files: Vec::new(),
            selected_device: None,
            show_settings: false,
            send_cancel,
            receive_cancel,
            local_ip,
            active_view: View::Files,
            incoming_request,
            active_transfer,
            tokio_handle,
        }
    }

    pub fn respond_to_incoming(&self, accepted: bool) {
        let req = self.incoming_request.lock().unwrap().take();
        if let Some(req) = req {
            req.respond(accepted);
        }
    }

    pub fn start_send_with_ctx(&mut self, ctx: &egui::Context) {
        if self.pending_files.is_empty() {
            return;
        }

        let device = {
            let devs = self.devices.lock().unwrap();
            match self.selected_device {
                Some(idx) if idx < devs.len() => Some(devs[idx].clone()),
                _ => None,
            }
        };

        let device = match device {
            Some(d) => d,
            None => {
                let mut st = self.transfer_state.lock().unwrap();
                st.add_log("No device selected".into());
                return;
            }
        };

        if self.active_transfer.load(Ordering::Relaxed) {
            let mut st = self.transfer_state.lock().unwrap();
            st.add_log("Send blocked — another transfer is already running".into());
            return;
        }

        self.send_cancel.store(false, Ordering::Relaxed);

        let pending = self.pending_files.clone();
        self.pending_files.clear();

        transfer::spawn_send(
            &self.tokio_handle,
            device.ip,
            device.port,
            pending,
            self.config.chunk_size,
            self.config.device_name.clone(),
            self.transfer_state.clone(),
            self.send_cancel.clone(),
            self.active_transfer.clone(),
            ctx.clone(),
        );
    }
}

impl eframe::App for LankerApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.07, 0.07, 0.07, 1.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ui::render_ui(self, ctx);

        let state = self.transfer_state.lock().unwrap();
        let has_incoming = self.incoming_request.lock().unwrap().is_some();
        if state.is_sending || state.is_receiving || has_incoming {
            ctx.request_repaint();
        }
    }

    fn ui(&mut self, _ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
    }
}
