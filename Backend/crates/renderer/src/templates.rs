use crate::*;
use askama::Template;

/// OOF Moment card template
#[derive(Template)]
#[template(source = r#\"<svg xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink' width='{{ width }}' height='{{ height }}'>
  <defs>
    {% if gradient %}
    <linearGradient id='grad' x1='0%' y1='0%' x2='100%' y2='100%'>
      <stop offset='0%' stop-color='#667eea'/>
      <stop offset='100%' stop-color='#764ba2'/>
    </linearGradient>
    {% endif %}
    <style>
      @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;600;800');
      .title { font: 800 {{ title_size }}px Inter, sans-serif; fill: {{ primary_text }}; }
      .subtitle { font: 600 {{ subtitle_size }}px Inter, sans-serif; fill: {{ secondary_text }}; }
      .badge { font: 700 {{ badge_size }}px Inter, sans-serif; fill: white; }
      .amount { font: 600 {{ amount_size }}px Inter, sans-serif; fill: {{ moment_color }}; }
      .meta { font: 400 {{ meta_size }}px Inter, sans-serif; fill: {{ secondary_text }}; }
    </style>
  </defs>

  <!-- Background -->
  <rect width='100%' height='100%' fill='{{ background }}'/>

  <!-- Moment badge -->
  <rect x='{{ badge_x }}' y='{{ badge_y }}' rx='{{ badge_radius }}' ry='{{ badge_radius }}' width='{{ badge_width }}' height='{{ badge_height }}' fill='{{ moment_color }}'/>
  <text x='{{ badge_text_x }}' y='{{ badge_text_y }}' class='badge' dominant-baseline='middle' text-anchor='middle'>{{ kind }}</text>

  <!-- Main title -->
  <text x='50%' y='{{ title_y }}%' class='title' dominant-baseline='middle' text-anchor='middle'>{{ title }}</text>

  <!-- Subtitle -->
  <text x='50%' y='{{ subtitle_y }}%' class='subtitle' dominant-baseline='middle' text-anchor='middle'>{{ subtitle }}</text>

  {% if amount %}
  <!-- Amount -->
  <text x='50%' y='{{ amount_y }}%' class='amount' dominant-baseline='middle' text-anchor='middle'>{{ amount }}</text>
  {% endif %}

  {% if wallet %}
  <!-- Wallet -->
  <text x='50%' y='{{ wallet_y }}%' class='meta' dominant-baseline='middle' text-anchor='middle'>{{ wallet_formatted }}</text>
  {% endif %}

  {% if timestamp %}
  <!-- Timestamp -->
  <text x='{{ width - 20 }}' y='{{ height - 20 }}' class='meta' dominant-baseline='bottom' text-anchor='end'>{{ timestamp }}</text>
  {% endif %}

  <!-- Decorative elements -->
  <circle cx='{{ decoration_x }}' cy='{{ decoration_y }}' r='{{ decoration_radius }}' fill='{{ moment_color }}' opacity='0.1'/>
  <circle cx='{{ decoration_x2 }}' cy='{{ decoration_y2 }}' r='{{ decoration_radius2 }}' fill='{{ moment_color }}' opacity='0.05'/>
</svg>\"#, ext = \"svg\")]
pub struct MomentCardTemplate {
    // Basic dimensions
    width: u32,
    height: u32,

    // Theme colors
    background: String,
    primary_text: String,
    secondary_text: String,
    moment_color: String,
    gradient: bool,

    // Content
    title: String,
    subtitle: String,
    kind: String,
    amount: Option<String>,
    wallet: Option<String>,
    wallet_formatted: Option<String>,
    timestamp: Option<String>,

    // Layout calculations
    title_size: u32,
    subtitle_size: u32,
    badge_size: u32,
    amount_size: u32,
    meta_size: u32,

    title_y: u32,
    subtitle_y: u32,
    amount_y: u32,
    wallet_y: u32,

    badge_x: u32,
    badge_y: u32,
    badge_width: u32,
    badge_height: u32,
    badge_radius: u32,
    badge_text_x: u32,
    badge_text_y: u32,

    decoration_x: u32,
    decoration_y: u32,
    decoration_radius: u32,
    decoration_x2: u32,
    decoration_y2: u32,
    decoration_radius2: u32,
}

impl MomentCardTemplate {
    pub fn new(moment: &MomentCard, theme: &CardTheme, width: u32, height: u32) -> Self {
        let (background, primary_text, secondary_text, _) = get_theme_colors(theme);
        let moment_color = get_moment_colors(&moment.kind);
        let gradient = matches!(theme, CardTheme::Gradient);

        // Scale font sizes based on card size
        let scale = (width as f32 / 1200.0).min(height as f32 / 630.0);
        let title_size = (72.0 * scale) as u32;
        let subtitle_size = (28.0 * scale) as u32;
        let badge_size = (22.0 * scale) as u32;
        let amount_size = (36.0 * scale) as u32;
        let meta_size = (18.0 * scale) as u32;

        // Layout calculations
        let badge_width = (220.0 * scale) as u32;
        let badge_height = (56.0 * scale) as u32;
        let badge_radius = (10.0 * scale) as u32;
        let badge_x = (60.0 * scale) as u32;
        let badge_y = (60.0 * scale) as u32;
        let badge_text_x = badge_x + badge_width / 2;
        let badge_text_y = badge_y + badge_height / 2;

        // Vertical positioning (percentages)
        let title_y = 40;
        let subtitle_y = if moment.amount.is_some() { 52 } else { 58 };
        let amount_y = 64;
        let wallet_y = if moment.amount.is_some() { 72 } else { 68 };

        // Decorative elements
        let decoration_x = width - (width / 4);
        let decoration_y = height / 4;
        let decoration_radius = (width / 8).min(height / 8);
        let decoration_x2 = width / 6;
        let decoration_y2 = height - (height / 3);
        let decoration_radius2 = decoration_radius / 2;

        let wallet_formatted = moment.wallet.as_ref().map(|w| format_wallet(w));

        Self {
            width,
            height,
            background,
            primary_text,
            secondary_text,
            moment_color,
            gradient,
            title: moment.title.clone(),
            subtitle: moment.subtitle.clone(),
            kind: moment.kind.clone(),
            amount: moment.amount.as_ref().map(|a| format_number(a)),
            wallet: moment.wallet.clone(),
            wallet_formatted,
            timestamp: moment.timestamp.clone(),
            title_size,
            subtitle_size,
            badge_size,
            amount_size,
            meta_size,
            title_y,
            subtitle_y,
            amount_y,
            wallet_y,
            badge_x,
            badge_y,
            badge_width,
            badge_height,
            badge_radius,
            badge_text_x,
            badge_text_y,
            decoration_x,
            decoration_y,
            decoration_radius,
            decoration_x2,
            decoration_y2,
            decoration_radius2,
        }
    }
}

/// Wallet summary card template
#[derive(Template)]
#[template(source = r#\"<svg xmlns='http://www.w3.org/2000/svg' width='{{ width }}' height='{{ height }}'>
  <defs>
    {% if gradient %}
    <linearGradient id='grad' x1='0%' y1='0%' x2='100%' y2='100%'>
      <stop offset='0%' stop-color='#667eea'/>
      <stop offset='100%' stop-color='#764ba2'/>
    </linearGradient>
    {% endif %}
    <style>
      @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;600;800');
      .title { font: 800 {{ title_size }}px Inter, sans-serif; fill: {{ primary_text }}; }
      .subtitle { font: 600 {{ subtitle_size }}px Inter, sans-serif; fill: {{ secondary_text }}; }
      .pnl { font: 800 {{ pnl_size }}px Inter, sans-serif; fill: {{ pnl_color }}; }
      .stat-label { font: 400 {{ stat_label_size }}px Inter, sans-serif; fill: {{ secondary_text }}; }
      .stat-value { font: 600 {{ stat_value_size }}px Inter, sans-serif; fill: {{ primary_text }}; }
      .meta { font: 400 {{ meta_size }}px Inter, sans-serif; fill: {{ secondary_text }}; }
    </style>
  </defs>

  <!-- Background -->
  <rect width='100%' height='100%' fill='{{ background }}'/>

  <!-- Title -->
  <text x='50%' y='{{ title_y }}' class='title' dominant-baseline='middle' text-anchor='middle'>Portfolio Summary</text>

  <!-- Wallet -->
  <text x='50%' y='{{ wallet_y }}' class='subtitle' dominant-baseline='middle' text-anchor='middle'>{{ wallet_formatted }}</text>

  <!-- Total P&L -->
  <text x='50%' y='{{ pnl_y }}' class='pnl' dominant-baseline='middle' text-anchor='middle'>{{ total_pnl_formatted }}</text>

  <!-- Stats grid -->
  <text x='{{ stat1_x }}' y='{{ stats_y }}' class='stat-label' text-anchor='middle'>Total Moments</text>
  <text x='{{ stat1_x }}' y='{{ stats_value_y }}' class='stat-value' text-anchor='middle'>{{ total_moments }}</text>

  <text x='{{ stat2_x }}' y='{{ stats_y }}' class='stat-label' text-anchor='middle'>Top Issue</text>
  <text x='{{ stat2_x }}' y='{{ stats_value_y }}' class='stat-value' text-anchor='middle'>{{ top_moment_type }}</text>

  <!-- Analysis period -->
  <text x='50%' y='{{ period_y }}' class='meta' dominant-baseline='middle' text-anchor='middle'>{{ analysis_period }}</text>

  <!-- Timestamp -->
  <text x='{{ width - 20 }}' y='{{ height - 20 }}' class='meta' dominant-baseline='bottom' text-anchor='end'>{{ timestamp }}</text>
</svg>\"#, ext = \"svg\")]
pub struct WalletSummaryCardTemplate {
    // Basic dimensions
    width: u32,
    height: u32,

    // Theme colors
    background: String,
    primary_text: String,
    secondary_text: String,
    pnl_color: String,
    gradient: bool,

    // Content
    wallet_formatted: String,
    total_pnl_formatted: String,
    total_moments: u32,
    top_moment_type: String,
    analysis_period: String,
    timestamp: String,

    // Layout calculations
    title_size: u32,
    subtitle_size: u32,
    pnl_size: u32,
    stat_label_size: u32,
    stat_value_size: u32,
    meta_size: u32,

    title_y: u32,
    wallet_y: u32,
    pnl_y: u32,
    stats_y: u32,
    stats_value_y: u32,
    period_y: u32,

    stat1_x: u32,
    stat2_x: u32,
}

impl WalletSummaryCardTemplate {
    pub fn new(summary: &WalletSummaryCard, theme: &CardTheme, width: u32, height: u32) -> Self {
        let (background, primary_text, secondary_text, _) = get_theme_colors(theme);
        let gradient = matches!(theme, CardTheme::Gradient);

        // Determine P&L color
        let pnl_color = if summary.total_pnl.starts_with('-') {
            \"#ff3355\".to_string() // Red for losses
        } else {
            \"#00d1b2\".to_string() // Green for gains
        };

        // Scale font sizes
        let scale = (width as f32 / 1200.0).min(height as f32 / 630.0);
        let title_size = (48.0 * scale) as u32;
        let subtitle_size = (24.0 * scale) as u32;
        let pnl_size = (64.0 * scale) as u32;
        let stat_label_size = (16.0 * scale) as u32;
        let stat_value_size = (24.0 * scale) as u32;
        let meta_size = (16.0 * scale) as u32;

        // Layout calculations
        let title_y = (height as f32 * 0.2) as u32;
        let wallet_y = (height as f32 * 0.3) as u32;
        let pnl_y = (height as f32 * 0.5) as u32;
        let stats_y = (height as f32 * 0.65) as u32;
        let stats_value_y = (height as f32 * 0.75) as u32;
        let period_y = (height as f32 * 0.85) as u32;

        let stat1_x = (width as f32 * 0.33) as u32;
        let stat2_x = (width as f32 * 0.67) as u32;

        Self {
            width,
            height,
            background,
            primary_text,
            secondary_text,
            pnl_color,
            gradient,
            wallet_formatted: format_wallet(&summary.wallet),
            total_pnl_formatted: format!(\"${}\", format_number(&summary.total_pnl)),
            total_moments: summary.total_moments,
            top_moment_type: summary.top_moment_type.clone(),
            analysis_period: summary.analysis_period.clone(),
            timestamp: summary.timestamp.clone(),
            title_size,
            subtitle_size,
            pnl_size,
            stat_label_size,
            stat_value_size,
            meta_size,
            title_y,
            wallet_y,
            pnl_y,
            stats_y,
            stats_value_y,
            period_y,
            stat1_x,
            stat2_x,
        }
    }
}

/// Leaderboard card template
#[derive(Template)]
#[template(source = r#\"<svg xmlns='http://www.w3.org/2000/svg' width='{{ width }}' height='{{ height }}'>
  <defs>
    {% if gradient %}
    <linearGradient id='grad' x1='0%' y1='0%' x2='100%' y2='100%'>
      <stop offset='0%' stop-color='#667eea'/>
      <stop offset='100%' stop-color='#764ba2'/>
    </linearGradient>
    {% endif %}
    <style>
      @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;600;800');
      .title { font: 800 {{ title_size }}px Inter, sans-serif; fill: {{ primary_text }}; }
      .subtitle { font: 600 {{ subtitle_size }}px Inter, sans-serif; fill: {{ secondary_text }}; }
      .rank { font: 800 {{ rank_size }}px Inter, sans-serif; fill: {{ accent_color }}; }
      .wallet { font: 600 {{ wallet_size }}px Inter, sans-serif; fill: {{ primary_text }}; }
      .value { font: 600 {{ value_size }}px Inter, sans-serif; fill: {{ secondary_text }}; }
      .meta { font: 400 {{ meta_size }}px Inter, sans-serif; fill: {{ secondary_text }}; }
    </style>
  </defs>

  <!-- Background -->
  <rect width='100%' height='100%' fill='{{ background }}'/>

  <!-- Title -->
  <text x='50%' y='{{ title_y }}' class='title' dominant-baseline='middle' text-anchor='middle'>{{ title }}</text>

  <!-- Period -->
  <text x='50%' y='{{ period_y }}' class='subtitle' dominant-baseline='middle' text-anchor='middle'>{{ period }}</text>

  <!-- Leaderboard entries -->
  {% for entry in entries %}
  <g transform='translate(0, {{ entry_y_base + loop.index0 * entry_spacing }})'>
    <!-- Rank -->
    <text x='{{ rank_x }}' y='0' class='rank' dominant-baseline='middle'>{{ entry.rank }}</text>

    <!-- Wallet -->
    <text x='{{ wallet_x }}' y='0' class='wallet' dominant-baseline='middle'>{{ entry.wallet }}</text>

    <!-- Value -->
    <text x='{{ value_x }}' y='0' class='value' dominant-baseline='middle' text-anchor='end'>{{ entry.value }}</text>
  </g>
  {% endfor %}

  <!-- Timestamp -->
  <text x='{{ width - 20 }}' y='{{ height - 20 }}' class='meta' dominant-baseline='bottom' text-anchor='end'>{{ timestamp }}</text>
</svg>\"#, ext = \"svg\")]
pub struct LeaderboardCardTemplate {
    // Basic dimensions
    width: u32,
    height: u32,

    // Theme colors
    background: String,
    primary_text: String,
    secondary_text: String,
    accent_color: String,
    gradient: bool,

    // Content
    title: String,
    period: String,
    entries: Vec<LeaderboardEntry>,
    timestamp: String,

    // Layout calculations
    title_size: u32,
    subtitle_size: u32,
    rank_size: u32,
    wallet_size: u32,
    value_size: u32,
    meta_size: u32,

    title_y: u32,
    period_y: u32,
    entry_y_base: u32,
    entry_spacing: u32,

    rank_x: u32,
    wallet_x: u32,
    value_x: u32,
}

impl LeaderboardCardTemplate {
    pub fn new(leaderboard: &LeaderboardCard, theme: &CardTheme, width: u32, height: u32) -> Self {
        let (background, primary_text, secondary_text, accent) = get_theme_colors(theme);
        let gradient = matches!(theme, CardTheme::Gradient);

        // Scale font sizes
        let scale = (width as f32 / 1200.0).min(height as f32 / 630.0);
        let title_size = (36.0 * scale) as u32;
        let subtitle_size = (20.0 * scale) as u32;
        let rank_size = (24.0 * scale) as u32;
        let wallet_size = (18.0 * scale) as u32;
        let value_size = (16.0 * scale) as u32;
        let meta_size = (14.0 * scale) as u32;

        // Layout calculations
        let title_y = (height as f32 * 0.15) as u32;
        let period_y = (height as f32 * 0.25) as u32;
        let entry_y_base = (height as f32 * 0.4) as u32;
        let available_height = (height as f32 * 0.45) as u32;
        let entry_spacing = if leaderboard.entries.len() > 0 {
            available_height / leaderboard.entries.len() as u32
        } else {
            40
        };

        let rank_x = (width as f32 * 0.1) as u32;
        let wallet_x = (width as f32 * 0.2) as u32;
        let value_x = (width as f32 * 0.9) as u32;

        // Format entries with truncated wallets
        let mut formatted_entries = Vec::new();
        for entry in &leaderboard.entries {
            formatted_entries.push(LeaderboardEntry {
                rank: entry.rank,
                wallet: format_wallet(&entry.wallet),
                value: format_number(&entry.value),
                metric: entry.metric.clone(),
            });
        }

        Self {
            width,
            height,
            background,
            primary_text,
            secondary_text,
            accent_color: \"#6c8cff\".to_string(),
            gradient,
            title: leaderboard.title.clone(),
            period: format!(\"Last {}\", leaderboard.period),
            entries: formatted_entries,
            timestamp: leaderboard.timestamp.clone(),
            title_size,
            subtitle_size,
            rank_size,
            wallet_size,
            value_size,
            meta_size,
            title_y,
            period_y,
            entry_y_base,
            entry_spacing,
            rank_x,
            wallet_x,
            value_x,
        }
    }
}
