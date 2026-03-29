use std::{sync::atomic::{AtomicU64, Ordering}, sync::Arc, path::PathBuf};
use tokio::io::AsyncWriteExt;

pub struct Metrics {
    dir_path: Option<PathBuf>,
    pub add_ops: AtomicU64,
    pub add_time: AtomicU64,
    pub list_ops: AtomicU64,
    pub list_time: AtomicU64,
    pub set_ops: AtomicU64,
    pub set_time: AtomicU64,
    pub get_ops: AtomicU64,
    pub get_time: AtomicU64,
    pub del_ops: AtomicU64,
    pub del_time: AtomicU64,
    pub event_batches: AtomicU64,
    pub event_closed: AtomicU64,
}

impl Metrics {
    pub fn new_from_env() -> Self {
        let dir_path = std::env::var("METRICS_DIR").ok().filter(|s| !s.is_empty()).map(PathBuf::from);
        Self {
            dir_path,
            add_ops: AtomicU64::new(0),
            add_time: AtomicU64::new(0),
            list_ops: AtomicU64::new(0),
            list_time: AtomicU64::new(0),
            set_ops: AtomicU64::new(0),
            set_time: AtomicU64::new(0),
            get_ops: AtomicU64::new(0),
            get_time: AtomicU64::new(0),
            del_ops: AtomicU64::new(0),
            del_time: AtomicU64::new(0),
            event_batches: AtomicU64::new(0),
            event_closed: AtomicU64::new(0),
        }
    }

    pub fn enabled(&self) -> bool {
        self.dir_path.is_some()
    }

    pub fn record_add(&self, dur: std::time::Duration) {
        if !self.enabled() { return; }
        self.add_ops.fetch_add(1, Ordering::Relaxed);
        self.add_time.fetch_add(dur.as_nanos() as u64, Ordering::Relaxed);
    }
    pub fn record_list(&self, dur: std::time::Duration) {
        if !self.enabled() { return; }
        self.list_ops.fetch_add(1, Ordering::Relaxed);
        self.list_time.fetch_add(dur.as_nanos() as u64, Ordering::Relaxed);
    }
    pub fn record_set(&self, dur: std::time::Duration) {
        if !self.enabled() { return; }
        self.set_ops.fetch_add(1, Ordering::Relaxed);
        self.set_time.fetch_add(dur.as_nanos() as u64, Ordering::Relaxed);
    }
    pub fn record_get(&self, dur: std::time::Duration) {
        if !self.enabled() { return; }
        self.get_ops.fetch_add(1, Ordering::Relaxed);
        self.get_time.fetch_add(dur.as_nanos() as u64, Ordering::Relaxed);
    }
    pub fn record_del(&self, dur: std::time::Duration) {
        if !self.enabled() { return; }
        self.del_ops.fetch_add(1, Ordering::Relaxed);
        self.del_time.fetch_add(dur.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn spawn_logger(metrics: Arc<Metrics>) {
        if !metrics.enabled() { return; }
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            const WARN_MS: f64 = 10.0;
            const ALERT_MS: f64 = 50.0;

            let colorize = |val_ms: f64, width: usize| {
                let base = format!("{:>width$.3}", val_ms, width = width);
                if val_ms >= ALERT_MS {
                    format!("\x1b[31m{}\x1b[0m", base)
                } else if val_ms >= WARN_MS {
                    format!("\x1b[33m{}\x1b[0m", base)
                } else {
                    format!("\x1b[32m{}\x1b[0m", base)
                }
            };

            // Resolve paths once outside the loop
            let (rt_path, hist_path) = {
                let dir = metrics.dir_path.as_ref().expect("metrics enabled implies dir_path");
                (dir.join("metrics_rt.txt"), dir.join("metrics_hist.csv"))
            };

            // Ensure directory exists
            if let Some(dir) = metrics.dir_path.as_ref() {
                if let Err(e) = tokio::fs::create_dir_all(dir).await {
                    log::warn!("metrics: failed to create dir {}: {e}", dir.display());
                    return;
                }
            }

            loop {
                interval.tick().await;
                let a_ops = metrics.add_ops.swap(0, Ordering::Relaxed);
                let a_time = metrics.add_time.swap(0, Ordering::Relaxed);
                let l_ops = metrics.list_ops.swap(0, Ordering::Relaxed);
                let l_time = metrics.list_time.swap(0, Ordering::Relaxed);
                let s_ops = metrics.set_ops.swap(0, Ordering::Relaxed);
                let s_time = metrics.set_time.swap(0, Ordering::Relaxed);
                let g_ops = metrics.get_ops.swap(0, Ordering::Relaxed);
                let g_time = metrics.get_time.swap(0, Ordering::Relaxed);
                let d_ops = metrics.del_ops.swap(0, Ordering::Relaxed);
                let d_time = metrics.del_time.swap(0, Ordering::Relaxed);
                let ev_batches = metrics.event_batches.swap(0, Ordering::Relaxed);
                let ev_closed = metrics.event_closed.swap(0, Ordering::Relaxed);

                let avg = |ops: u64, ns: u64| if ops == 0 { 0.0 } else { ns as f64 / ops as f64 / 1e6 };
                if let Some(_dir) = metrics.dir_path.as_ref() {
                    const OP_W: usize = 10;
                    const COUNT_W: usize = 12;
                    const AVG_W: usize = 12;
                    const EVT_W: usize = 18;

                    let hdr = format!(
                        "┌{h1}┬{h2}┬{h3}┐\n│{op:<w1$}│{cnt:<w2$}│{avg:<w3$}│\n├{h1}┼{h2}┼{h3}┤\n",
                        h1="─".repeat(OP_W),
                        h2="─".repeat(COUNT_W),
                        h3="─".repeat(AVG_W),
                        op="op",
                        cnt="count",
                        avg="avg_ms",
                        w1=OP_W,
                        w2=COUNT_W,
                        w3=AVG_W,
                    );

                    let body = format!(
                        "│{:<w1$}│{a_ops:>w2$}│{add_avg}│\n│{:<w1$}│{l_ops:>w2$}│{list_avg}│\n│{:<w1$}│{s_ops:>w2$}│{set_avg}│\n│{:<w1$}│{g_ops:>w2$}│{get_avg}│\n│{:<w1$}│{d_ops:>w2$}│{del_avg}│\n",
                        "add", "list", "set", "get", "del",
                        a_ops=a_ops,
                        l_ops=l_ops,
                        s_ops=s_ops,
                        g_ops=g_ops,
                        d_ops=d_ops,
                        add_avg=colorize(avg(a_ops, a_time), AVG_W),
                        list_avg=colorize(avg(l_ops, l_time), AVG_W),
                        set_avg=colorize(avg(s_ops, s_time), AVG_W),
                        get_avg=colorize(avg(g_ops, g_time), AVG_W),
                        del_avg=colorize(avg(d_ops, d_time), AVG_W),
                        w1=OP_W,
                        w2=COUNT_W,
                    );

                    let footer = format!(
                        "└{h1}┴{h2}┴{h3}┘\n",
                        h1="─".repeat(OP_W),
                        h2="─".repeat(COUNT_W),
                        h3="─".repeat(AVG_W),
                    );

                    let events = format!(
                        "┌{h4}┬{h2}┐\n│{ev_hdr:<w4$}│{cnt:<w2$}│\n├{h4}┼{h2}┤\n│{b_hdr:<w4$}│{ev_batches:>w2$}│\n│{c_hdr:<w4$}│{ev_closed:>w2$}│\n└{h4}┴{h2}┘\n",
                        h4="─".repeat(EVT_W),
                        h2="─".repeat(COUNT_W),
                        ev_hdr="events",
                        b_hdr="batches",
                        c_hdr="closed",
                        cnt="count",
                        ev_batches=ev_batches,
                        ev_closed=ev_closed,
                        w4=EVT_W,
                        w2=COUNT_W,
                    );

                    let table = format!("\n{hdr}{body}{footer}{events}", hdr=hdr, body=body, footer=footer, events=events);
                    if let Err(e) = tokio::fs::write(&rt_path, table).await {
                        log::warn!("metrics: failed to write file {}: {e}", rt_path.display());
                    }

                    // Append historical CSV
                    let ts = chrono::Utc::now().to_rfc3339();
                    let add_avg_v = avg(a_ops, a_time);
                    let list_avg_v = avg(l_ops, l_time);
                    let set_avg_v = avg(s_ops, s_time);
                    let get_avg_v = avg(g_ops, g_time);
                    let del_avg_v = avg(d_ops, d_time);

                    let header_needed = match tokio::fs::metadata(&hist_path).await {
                        Ok(meta) => meta.len() == 0,
                        Err(_) => true,
                    };

                    match tokio::fs::OpenOptions::new().append(true).create(true).open(&hist_path).await {
                        Ok(mut file) => {
                            if header_needed {
                                let _ = file.write_all(b"timestamp,add_ops,add_avg_ms,list_ops,list_avg_ms,set_ops,set_avg_ms,get_ops,get_avg_ms,del_ops,del_avg_ms,event_batches,event_closed\n").await;
                            }
                            let line = format!(
                                "{ts},{a_ops},{add_avg_v:.3},{l_ops},{list_avg_v:.3},{s_ops},{set_avg_v:.3},{g_ops},{get_avg_v:.3},{d_ops},{del_avg_v:.3},{ev_batches},{ev_closed}\n",
                            );
                            if let Err(e) = file.write_all(line.as_bytes()).await {
                                log::warn!("metrics: failed to write hist {}: {e}", hist_path.display());
                            }
                        }
                        Err(e) => log::warn!("metrics: failed to open hist {}: {e}", hist_path.display()),
                    }
                }
            }
        });
    }
}
