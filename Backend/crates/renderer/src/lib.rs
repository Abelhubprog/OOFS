use anyhow::Result;
use askama::Template;
use resvg::render;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use shared::store::ObjectStore;
use std::sync::Arc;
use time::OffsetDateTime;
use tiny_skia::Pixmap;
use tracing::{error, info, instrument};
use usvg::{Options, TreeParsing};

mod templates;
use templates::*;

// Re-export template types
pub use templates::{LeaderboardCardTemplate, MomentCardTemplate, WalletSummaryCardTemplate};

/// Card themes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CardTheme {
    Dark,
    Light,
    Gradient,
}

/// Card sizes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CardSize {
    /// 1200x630 - Twitter/LinkedIn card size
    Social,
    /// 800x400 - Medium card size
    Medium,
    /// 600x314 - Small card size
    Small,
}

impl CardSize {
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            CardSize::Social => (1200, 630),
            CardSize::Medium => (800, 400),
            CardSize::Small => (600, 314),
        }
    }
}

/// OOF Moment card data
#[derive(Clone, Serialize, Deserialize)]
pub struct MomentCard {
    pub title: String,
    pub subtitle: String,
    pub kind: String,
    pub primary: String,
    pub wallet: Option<String>,
    pub amount: Option<String>,
    pub percentage: Option<String>,
    pub timestamp: Option<String>,
    pub token_symbol: Option<String>,
}

/// Wallet summary card data
#[derive(Clone, Serialize, Deserialize)]
pub struct WalletSummaryCard {
    pub wallet: String,
    pub total_pnl: String,
    pub total_moments: u32,
    pub top_moment_type: String,
    pub analysis_period: String,
    pub timestamp: String,
}

/// Leaderboard card data
#[derive(Clone, Serialize, Deserialize)]
pub struct LeaderboardCard {
    pub title: String,
    pub period: String,
    pub entries: Vec<LeaderboardEntry>,
    pub timestamp: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub rank: u32,
    pub wallet: String,
    pub value: String,
    pub metric: String,
}

/// Card renderer service
pub struct CardRenderer {
    store: Option<Arc<dyn ObjectStore>>,
}

impl CardRenderer {
    pub fn new(store: Option<Arc<dyn ObjectStore>>) -> Self {
        Self { store }
    }

    /// Render OOF moment card to PNG
    #[instrument(skip(self, moment))]
    pub async fn render_moment_card(
        &self,
        moment: &MomentCard,
        theme: CardTheme,
        size: CardSize,
    ) -> Result<Vec<u8>> {
        let (width, height) = size.dimensions();
        let template = MomentCardTemplate::new(moment, &theme, width, height);
        let svg = template.render()?;

        let png_data = render_svg_to_png(&svg, width, height)?;

        // Optionally store in object storage
        if let Some(store) = &self.store {
            let key = format!(
                "cards/moments/{}-{:?}-{:?}.png",
                moment.kind.to_lowercase(),
                theme,
                size
            );
            if let Err(e) = store.put(&key, &png_data).await {
                error!("Failed to store moment card: {}", e);
            }
        }

        Ok(png_data)
    }

    /// Render wallet summary card to PNG
    #[instrument(skip(self, summary))]
    pub async fn render_wallet_summary_card(
        &self,
        summary: &WalletSummaryCard,
        theme: CardTheme,
        size: CardSize,
    ) -> Result<Vec<u8>> {
        let (width, height) = size.dimensions();
        let template = WalletSummaryCardTemplate::new(summary, &theme, width, height);
        let svg = template.render()?;

        let png_data = render_svg_to_png(&svg, width, height)?;

        // Optionally store in object storage
        if let Some(store) = &self.store {
            let key = format!(
                "cards/wallets/{}-{:?}-{:?}.png",
                summary.wallet, theme, size
            );
            if let Err(e) = store.put(&key, &png_data).await {
                error!("Failed to store wallet summary card: {}", e);
            }
        }

        Ok(png_data)
    }

    /// Render leaderboard card to PNG
    #[instrument(skip(self, leaderboard))]
    pub async fn render_leaderboard_card(
        &self,
        leaderboard: &LeaderboardCard,
        theme: CardTheme,
        size: CardSize,
    ) -> Result<Vec<u8>> {
        let (width, height) = size.dimensions();
        let template = LeaderboardCardTemplate::new(leaderboard, &theme, width, height);
        let svg = template.render()?;

        let png_data = render_svg_to_png(&svg, width, height)?;

        // Optionally store in object storage
        if let Some(store) = &self.store {
            let key = format!(
                "cards/leaderboard/{}-{:?}-{:?}.png",
                leaderboard.period, theme, size
            );
            if let Err(e) = store.put(&key, &png_data).await {
                error!("Failed to store leaderboard card: {}", e);
            }
        }

        Ok(png_data)
    }
}

/// Render SVG string to PNG bytes
fn render_svg_to_png(svg: &str, width: u32, height: u32) -> Result<Vec<u8>> {
    let opt = Options::default();
    let tree =
        usvg::Tree::from_str(svg, &opt).map_err(|e| anyhow::anyhow!("SVG parse error: {}", e))?;

    let mut pixmap =
        Pixmap::new(width, height).ok_or_else(|| anyhow::anyhow!("Failed to create pixmap"))?;

    render(&tree, resvg::FitTo::Original, pixmap.as_mut())
        .ok_or_else(|| anyhow::anyhow!("SVG render failed"))?;

    let png = pixmap
        .encode_png()
        .map_err(|e| anyhow::anyhow!("PNG encoding failed: {}", e))?;

    Ok(png)
}

/// Get theme colors
fn get_theme_colors(theme: &CardTheme) -> (String, String, String, String) {
    match theme {
        CardTheme::Dark => (
            "#0b0f1a".to_string(), // background
            "#ffffff".to_string(), // primary text
            "#d0d4db".to_string(), // secondary text
            "#1a1f2e".to_string(), // accent
        ),
        CardTheme::Light => (
            "#ffffff".to_string(), // background
            "#0b0f1a".to_string(), // primary text
            "#6b7280".to_string(), // secondary text
            "#f3f4f6".to_string(), // accent
        ),
        CardTheme::Gradient => (
            "url(#grad)".to_string(), // background gradient
            "#ffffff".to_string(),    // primary text
            "#e5e7eb".to_string(),    // secondary text
            "#374151".to_string(),    // accent
        ),
    }
}

/// Get moment kind colors
fn get_moment_colors(kind: &str) -> String {
    match kind.to_lowercase().as_str() {
        "sold_too_early" | "s2e" => "#ff7d00".to_string(), // Orange
        "bag_holder_drawdown" | "bhd" => "#ff3355".to_string(), // Red
        "bad_route" => "#00d1b2".to_string(),              // Teal
        "idle_yield" | "idle" => "#6c8cff".to_string(),    // Blue
        "rug" => "#dc2626".to_string(),                    // Dark red
        _ => "#6b7280".to_string(),                        // Gray
    }
}

/// Format wallet address for display
fn format_wallet(wallet: &str) -> String {
    if wallet.len() > 8 {
        format!("{}...{}", &wallet[0..4], &wallet[wallet.len() - 4..])
    } else {
        wallet.to_string()
    }
}

/// Format large numbers with abbreviations
fn format_number(value: &str) -> String {
    if let Ok(num) = value.parse::<f64>() {
        if num.abs() >= 1_000_000_000.0 {
            format!("{:.1}B", num / 1_000_000_000.0)
        } else if num.abs() >= 1_000_000.0 {
            format!("{:.1}M", num / 1_000_000.0)
        } else if num.abs() >= 1_000.0 {
            format!("{:.1}K", num / 1_000.0)
        } else {
            format!("{:.2}", num)
        }
    } else {
        value.to_string()
    }
}

// Convenience functions for backward compatibility

/// Simple function to render a moment card (backward compatibility)
pub fn render_moment_card(moment: &MomentCard, theme: &str, size: &str) -> Result<Vec<u8>> {
    let card_theme = match theme {
        "light" => CardTheme::Light,
        "gradient" => CardTheme::Gradient,
        _ => CardTheme::Dark,
    };

    let card_size = match size {
        "800x400" => CardSize::Medium,
        "600x314" => CardSize::Small,
        _ => CardSize::Social,
    };

    let renderer = CardRenderer::new(None);

    // Use tokio runtime for async function
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(renderer.render_moment_card(moment, card_theme, card_size))
}

/// Create a moment card from basic parameters
pub fn create_moment_card(
    title: String,
    subtitle: String,
    kind: String,
    wallet: Option<String>,
    amount: Option<String>,
    percentage: Option<String>,
) -> MomentCard {
    MomentCard {
        title,
        subtitle,
        kind,
        primary: String::new(), // Will be determined by kind
        wallet,
        amount,
        percentage,
        timestamp: Some(
            OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
        ),
        token_symbol: None,
    }
}

/// Create a wallet summary card from basic parameters
pub fn create_wallet_summary_card(
    wallet: String,
    total_pnl: String,
    total_moments: u32,
    top_moment_type: String,
    analysis_period: String,
) -> WalletSummaryCard {
    WalletSummaryCard {
        wallet,
        total_pnl,
        total_moments,
        top_moment_type,
        analysis_period,
        timestamp: OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default(),
    }
}

/// Create a leaderboard card from basic parameters
pub fn create_leaderboard_card(
    title: String,
    period: String,
    entries: Vec<LeaderboardEntry>,
) -> LeaderboardCard {
    LeaderboardCard {
        title,
        period,
        entries,
        timestamp: OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default(),
    }
}
