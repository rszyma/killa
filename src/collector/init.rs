use std::sync::{mpsc, Arc};

use bottom::app::DataFilters;
use bottom::create_collection_thread;
use bottom::data_collection::temperature::TemperatureType;

pub fn init_collector() -> std::sync::mpsc::Receiver<bottom::event::BottomEvent> {
    let cancellation_token =
        Arc::new(bottom::utils::cancellation_token::CancellationToken::default());

    let (tx, rx) = mpsc::channel(); // use iced mpsc?

    let update_rate = 500; // min is 250 i think

    let btm_config = bottom::app::AppConfigFields {
        update_rate,
        temperature_type: TemperatureType::Celsius,
        show_average_cpu: true,
        use_dot: true,
        cpu_left_legend: true,
        use_current_cpu_total: true,
        unnormalized_cpu: false,
        use_basic_mode: false,
        default_time_value: 30 * 1000, // 30s
        time_interval: 1000,
        hide_time: true,
        autohide_time: false,
        use_old_network_legend: true,
        table_gap: 5,
        disable_click: true,
        enable_gpu: false,
        enable_cache_memory: false, // fixme: not sure what's this
        show_table_scroll_position: false,
        is_advanced_kill: false,
        memory_legend_position: None,
        network_legend_position: None,
        network_scale_type: bottom::app::AxisScaling::Linear,
        network_unit_type: bottom::utils::data_units::DataUnit::Bit,
        network_use_binary_prefix: false,
        retention_ms: 600000,
        dedicated_average_row: false,
    };

    // Set up the event loop thread.
    // Set it up early to speed up first access to data.
    let _ = create_collection_thread(
        tx.clone(),
        mpsc::channel().1, // ignore msg channel for now
        cancellation_token.clone(),
        &btm_config,
        DataFilters {
            disk_filter: None,
            mount_filter: None,
            temp_filter: None,
            net_filter: None,
        },
        bottom::app::layout_manager::UsedWidgets {
            use_cpu: true,
            use_mem: true,
            use_cache: true,
            use_gpu: true,
            use_net: true,
            use_proc: true,
            use_disk: true,
            use_temp: true,
            use_battery: true,
        },
    );

    rx
}
