#![no_std]
#![forbid(unsafe_code)]

//! # embassy-st7789v-plot
//!
//! Moteur de tracé de graphiques cartésiens (X, Y) adaptatifs et configurables
//! pour écrans TFT LCD ST7789V, s'appuyant sur la structure `Graphics`.
//!
//! ## Caractéristiques principales
//!
//! - **`#![no_std]` + `#![forbid(unsafe_code)]`** : Sûr et embarqué
//! - **Zéro allocation dynamique** : Buffers statiques uniquement (ring buffer)
//! - **API async** : Basée sur Embassy pour ST7789V
//! - **Axes configurables** : Graduations statiques avec labels personnalisés
//! - **Grille adaptative** : Grille horizontale/verticale pour meilleure lisibilité
//! - **Historique circulaire** : Jusqu'à 240 points (limité par la largeur écran)
//! - **Protection des bordures** : Les labels de graduations restent à l'écran
//!
//! ## Structures principales
//!
//! - [`Graphics`] : Contexte graphique pour les primitives de dessin
//! - [`AxisConfig`] : Configuration d'un axe (min, max, pas de graduation, label)
//! - [`PlotConfig`] : Configuration complète du graphique (position, marges, couleurs)
//! - [`LineChart`] : Gestionnaire du graphique avec données historiques
//!
//! ## Exemple d'utilisation
//!
//! ```no_run
//! # use embassy_st7789v::{Color, St7789v, NoPin};
//! # use embedded_hal::digital::OutputPin;
//! # use embedded_hal_async::spi::SpiDevice;
//! use embassy_st7789v_plot::{Graphics, AxisConfig, PlotConfig, LineChart};
//!
//! # async fn example<SPI, DC>(display: &mut St7789v<SPI, DC, NoPin>)
//! # where
//! #     SPI: SpiDevice,
//! #     DC: OutputPin,
//! # {
//! // Créer les configurations d'axes
//! let x_axis = AxisConfig::new(0.0, 10.0, 2.0, b"Time (s)");
//! let y_axis = AxisConfig::new(0.0, 100.0, 20.0, b"Temp (C)");
//!
//! // Créer la configuration complète du graphique
//! let config = PlotConfig {
//!     x: 10,
//!     y: 10,
//!     width: 220,
//!     height: 200,
//!     margin_left: 30,
//!     margin_right: 10,
//!     margin_top: 10,
//!     margin_bottom: 30,
//!     x_axis,
//!     y_axis,
//!     bg_color: Color::BLACK,
//!     line_color: Color::GREEN,
//!     axis_color: Color::WHITE,
//!     grid_color: Color::from_rgb(64, 64, 64),
//!     text_color: Color::WHITE,
//!     label_color: Color::CYAN,
//! };
//!
//! // Créer le gestionnaire de graphique (N = 100 points max)
//! let mut chart: LineChart<100> = LineChart::new(config);
//!
//! // Ajouter des données
//! chart.push(45.2);
//! chart.push(47.8);
//! chart.push(52.1);
//!
//! // Afficher le graphique
//! let mut gfx = Graphics::new_no_rst(display);
//! chart.render(&mut gfx).await;
//! # }
//! ```
//!
//! ## API asynchrone
//!
//! Tous les appels de rendu (`render`, `line`, `pixel`) sont asynchrones pour permettre
//! une meilleure intégration avec l'écosystème Embassy et éviter les blocages lors
//! de la communication SPI.

use embassy_st7789v::{Color, NoPin, St7789v, SCREEN_H, SCREEN_W};
use embedded_hal::digital::OutputPin;
use embedded_hal_async::spi::SpiDevice;

/// Taille maximale de l'historique des données du graphique.
pub const PLOT_HISTORY_LIMIT: usize = 240;

// ─────────────────────────────────────────────────────────────────────────────
// Contexte graphique embarqué (repris de tes primitives)
// ─────────────────────────────────────────────────────────────────────────────

pub struct Graphics<'a, SPI, DC, RST = NoPin>
where
    SPI: SpiDevice,
    DC: OutputPin,
    RST: OutputPin,
{
    /// Référence vers l'affichage ST7789V
    pub display: &'a mut St7789v<SPI, DC, RST>,
}

impl<'a, SPI, DC> Graphics<'a, SPI, DC, NoPin>
where
    SPI: SpiDevice,
    DC: OutputPin,
{
    /// Crée un nouveau contexte graphique sans broche RST.
    ///
    /// # Arguments
    ///
    /// * `display` - Référence mutable vers l'affichage ST7789V
    ///
    /// # Exemple
    ///
    /// ```no_run
    /// # use embassy_st7789v::{St7789v, NoPin};
    /// # use embedded_hal::digital::OutputPin;
    /// # use embedded_hal_async::spi::SpiDevice;
    /// # async fn example<SPI, DC>(display: &mut St7789v<SPI, DC, NoPin>) where SPI: SpiDevice, DC: OutputPin {
    /// use embassy_st7789v_plot::Graphics;
    ///
    /// let mut gfx = Graphics::new_no_rst(display);
    /// # }
    /// ```
    #[inline]
    pub fn new_no_rst(display: &'a mut St7789v<SPI, DC, NoPin>) -> Self {
        Self { display }
    }
}

impl<'a, SPI, DC, RST> Graphics<'a, SPI, DC, RST>
where
    SPI: SpiDevice,
    DC: OutputPin,
    RST: OutputPin,
{
    /// Crée un nouveau contexte graphique avec broche RST.
    ///
    /// # Arguments
    ///
    /// * `display` - Référence mutable vers l'affichage ST7789V
    ///
    /// # Exemple
    ///
    /// ```no_run
    /// # use embassy_st7789v::St7789v;
    /// # use embedded_hal::digital::OutputPin;
    /// # use embedded_hal_async::spi::SpiDevice;
    /// # async fn example<SPI, DC, RST>(display: &mut St7789v<SPI, DC, RST>) where SPI: SpiDevice, DC: OutputPin, RST: OutputPin {
    /// use embassy_st7789v_plot::Graphics;
    ///
    /// let mut gfx = Graphics::new(display);
    /// # }
    /// ```
    #[inline]
    pub fn new(display: &'a mut St7789v<SPI, DC, RST>) -> Self {
        Self { display }
    }

    /// Trace un pixel à la position (x, y) avec la couleur donnée.
    ///
    /// Les coordonnées négatives ou en dehors de l'écran sont ignorées silencieusement.
    ///
    /// # Arguments
    ///
    /// * `x` - Coordonnée X (peut être négative ou hors écran)
    /// * `y` - Coordonnée Y (peut être négative ou hors écran)
    /// * `color` - Couleur du pixel
    ///
    /// # Exemple
    ///
    /// ```no_run
    /// # use embassy_st7789v::{Color, St7789v};
    /// # use embedded_hal::digital::OutputPin;
    /// # use embedded_hal_async::spi::SpiDevice;
    /// # async fn example<SPI, DC, RST>(display: &mut St7789v<SPI, DC, RST>) where SPI: SpiDevice, DC: OutputPin, RST: OutputPin {
    /// use embassy_st7789v_plot::Graphics;
    ///
    /// let mut gfx = Graphics::new(display);
    /// gfx.pixel(100, 150, Color::GREEN).await;
    /// # }
    /// ```
    #[inline(always)]
    pub async fn pixel(&mut self, x: i32, y: i32, color: Color) {
        if x >= 0 && y >= 0 && x < SCREEN_W as i32 && y < SCREEN_H as i32 {
            let _ = self.display.draw_pixel(x as u16, y as u16, color).await;
        }
    }
}

/// Trace une ligne entre deux points en utilisant l'algorithme de Bresenham.
///
/// Cette fonction utilise l'algorithme de Bresenham pour tracer une ligne
/// entre les points (x0, y0) et (x1, y1). Elle gère correctement les lignes
/// en dehors de l'écran via la vérification de limites dans [`Graphics::pixel`].
///
/// # Arguments
///
/// * `gfx` - Contexte graphique
/// * `x0` - Coordonnée X du point de départ
/// * `y0` - Coordonnée Y du point de départ
/// * `x1` - Coordonnée X du point d'arrivée
/// * `y1` - Coordonnée Y du point d'arrivée
/// * `color` - Couleur de la ligne
///
/// # Exemple
///
/// ```no_run
/// # use embassy_st7789v::{Color, St7789v};
/// # use embedded_hal::digital::OutputPin;
/// # use embedded_hal_async::spi::SpiDevice;
/// # async fn example<SPI, DC, RST>(display: &mut St7789v<SPI, DC, RST>) where SPI: SpiDevice, DC: OutputPin, RST: OutputPin {
/// use embassy_st7789v_plot::{Graphics, line};
///
/// let mut gfx = Graphics::new(display);
/// line(&mut gfx, 10, 10, 100, 50, Color::BLUE).await;
/// # }
/// ```
pub async fn line<SPI, DC, RST>(
    gfx: &mut Graphics<'_, SPI, DC, RST>,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    color: Color,
) where
    SPI: SpiDevice,
    DC: OutputPin,
    RST: OutputPin,
{
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        gfx.pixel(x0, y0, color).await;
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Configuration des Axes
// ─────────────────────────────────────────────────────────────────────────────

/// Définit un axe avec graduation statique fixe.
///
/// Cette structure configure un axe du graphique avec une plage de valeurs,
/// un pas de graduation régulier et un label descriptif.
///
/// # Champs
///
/// * `start` - Valeur minimale de l'axe (ex: 0.0)
/// * `end` - Valeur maximale de l'axe (ex: 10.0)
/// * `step` - Espacement régulier entre graduations (ex: 1.0)
/// * `label` - Label texte affiché le long de l'axe (ex: b"Temp (C)")
///
/// # Exemple
///
/// ```
/// use embassy_st7789v_plot::AxisConfig;
///
/// // Axe des temps de 0 à 60 secondes avec graduations tous les 10s
/// let time_axis = AxisConfig::new(0.0, 60.0, 10.0, b"Time (s)");
/// assert!(time_axis.is_valid());
/// assert_eq!(time_axis.tick_count(), 7); // 0, 10, 20, 30, 40, 50, 60
///
/// // Axe de température de -10 à 50°C avec graduations tous les 10°C
/// let temp_axis = AxisConfig::new(-10.0, 50.0, 10.0, b"Temp (C)");
/// assert_eq!(temp_axis.tick_count(), 7);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct AxisConfig {
    pub start: f32,
    pub end: f32,
    pub step: f32,
    pub label: &'static [u8],
}

impl AxisConfig {
    /// Crée une nouvelle configuration d'axe.
    ///
    /// # Arguments
    ///
    /// * `start` - Valeur minimale (doit être < `end`)
    /// * `end` - Valeur maximale (doit être > `start`)
    /// * `step` - Pas de graduation (doit être > 0)
    /// * `label` - Label statique affiché (par exemple b"Temp (C)")
    ///
    /// # Panics
    ///
    /// Ne paniquera pas ici, mais utilisez [`is_valid`](Self::is_valid) après construction
    /// pour vérifier la cohérence.
    pub const fn new(start: f32, end: f32, step: f32, label: &'static [u8]) -> Self {
        Self { start, end, step, label }
    }

    /// Vérifie la cohérence de la configuration.
    ///
    /// Retourne `true` si :
    /// - `step` > 0.0
    /// - `end` > `start`
    ///
    /// # Exemple
    ///
    /// ```
    /// use embassy_st7789v_plot::AxisConfig;
    ///
    /// let valid = AxisConfig::new(0.0, 10.0, 1.0, b"X");
    /// assert!(valid.is_valid());
    ///
    /// let invalid = AxisConfig::new(10.0, 0.0, 1.0, b"X");
    /// assert!(!invalid.is_valid());
    /// ```
    pub fn is_valid(&self) -> bool {
        self.step > 0.0 && self.end > self.start
    }

    /// Calcule le nombre de graduations (incluant start et end).
    ///
    /// En `no_std`, le cast direct remplace `f32::floor()`.
    ///
    /// # Retour
    ///
    /// Nombre de ticks incluant les extrémités, ou 0 si la configuration est invalide.
    ///
    /// # Exemple
    ///
    /// ```
    /// use embassy_st7789v_plot::AxisConfig;
    ///
    /// let axis = AxisConfig::new(0.0, 10.0, 2.0, b"X");
    /// assert_eq!(axis.tick_count(), 6); // 0, 2, 4, 6, 8, 10
    /// ```
    pub fn tick_count(&self) -> usize {
        if !self.is_valid() {
            return 0;
        }
        let count = ((self.end - self.start) / self.step) as usize;
        count + 1
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Configuration et Structure de Traçage
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration complète du tracé graphique.
///
/// Cette structure définit tous les paramètres visuels et géométriques du graphique :
/// position, taille, marges, axes, et palette de couleurs.
///
/// # Champs
///
/// * `x`, `y` - Position du coin haut-gauche du graphique (en pixels)
/// * `width`, `height` - Dimensions du graphique (en pixels)
/// * `margin_*` - Marges internes pour les axes et labels
/// * `x_axis`, `y_axis` - Configurations des deux axes
/// * `*_color` - Couleurs pour le fond, les lignes, axes, grille, texte, labels
///
/// # Exemple
///
/// ```
/// use embassy_st7789v::{Color, SCREEN_W, SCREEN_H};
/// use embassy_st7789v_plot::{AxisConfig, PlotConfig};
///
/// let x_axis = AxisConfig::new(0.0, 10.0, 2.0, b"Time (s)");
/// let y_axis = AxisConfig::new(0.0, 100.0, 20.0, b"Value");
///
/// let config = PlotConfig {
///     x: 10,
///     y: 10,
///     width: 220,
///     height: 200,
///     margin_left: 40,
///     margin_right: 10,
///     margin_top: 10,
///     margin_bottom: 30,
///     x_axis,
///     y_axis,
///     bg_color: Color::BLACK,
///     line_color: Color::GREEN,
///     axis_color: Color::WHITE,
///     grid_color: Color::from_rgb(64, 64, 64),
///     text_color: Color::WHITE,
///     label_color: Color::CYAN,
/// };
/// ```
#[derive(Clone, Copy, Debug)]
pub struct PlotConfig {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub margin_left: i32,
    pub margin_right: i32,
    pub margin_top: i32,
    pub margin_bottom: i32,
    pub x_axis: AxisConfig,
    pub y_axis: AxisConfig,
    pub bg_color: Color,
    pub line_color: Color,
    pub axis_color: Color,
    pub grid_color: Color,
    pub text_color: Color,
    pub label_color: Color,
}

/// Gestionnaire du graphique avec axes statiques configurables.
///
/// `LineChart<N>` maintient un historique circulaire de N points de données et
/// gère le rendu du graphique avec grille, axes, graduations et labels.
///
/// # Paramètre générique
///
/// * `N` - Nombre maximum de points historiques (doit être ≤ `PLOT_HISTORY_LIMIT` = 240)
///
/// # Fonctionnement interne
///
/// - **Ring buffer** : Les données sont stockées dans un tableau fixe avec un pointeur
///   `head` qui tourne. Quand le buffer est plein, les nouvelles données écrasent les
///   plus anciennes.
/// - **Historique** : Seuls les N points les plus récents sont affichés.
/// - **Rendu** : Les données sont converties en pixels via `scale_x()` et `scale_y()`,
///   puis connectées par des lignes (Bresenham).
///
/// # Exemple
///
/// ```no_run
/// # use embassy_st7789v::{Color, St7789v, NoPin};
/// # use embedded_hal::digital::OutputPin;
/// # use embedded_hal_async::spi::SpiDevice;
/// use embassy_st7789v_plot::{Graphics, AxisConfig, PlotConfig, LineChart};
///
/// # async fn example<SPI, DC>(display: &mut St7789v<SPI, DC, NoPin>) where SPI: SpiDevice, DC: OutputPin {
/// // Créer la config
/// let config = PlotConfig {
///     x: 10, y: 10, width: 220, height: 200,
///     margin_left: 40, margin_right: 10, margin_top: 10, margin_bottom: 30,
///     x_axis: AxisConfig::new(0.0, 10.0, 2.0, b"Time"),
///     y_axis: AxisConfig::new(0.0, 100.0, 20.0, b"Value"),
///     bg_color: Color::BLACK,
///     line_color: Color::GREEN,
///     axis_color: Color::WHITE,
///     grid_color: Color::from_rgb(64, 64, 64),
///     text_color: Color::WHITE,
///     label_color: Color::CYAN,
/// };
///
/// // Créer et utiliser le graphique
/// let mut chart: LineChart<100> = LineChart::new(config);
/// chart.push(10.5);
/// chart.push(12.3);
/// chart.push(11.8);
///
/// let mut gfx = Graphics::new_no_rst(display);
/// chart.render(&mut gfx).await;
/// # }
/// ```
pub struct LineChart<const N: usize> {
    config: PlotConfig,
    data: [f32; N],
    head: usize,
    count: usize,
    plot_x: i32,
    plot_y: i32,
    plot_w: i32,
    plot_h: i32,
}

impl<const N: usize> LineChart<N> {
    /// Crée un nouveau gestionnaire de graphique.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration complète du graphique
    ///
    /// # Panics
    ///
    /// Panique si :
    /// - N > `PLOT_HISTORY_LIMIT` (240)
    /// - La configuration d'axe X est invalide
    /// - La configuration d'axe Y est invalide
    ///
    /// # Exemple
    ///
    /// ```
    /// use embassy_st7789v::{Color};
    /// use embassy_st7789v_plot::{AxisConfig, PlotConfig, LineChart};
    ///
    /// let config = PlotConfig {
    ///     x: 10, y: 10, width: 220, height: 200,
    ///     margin_left: 40, margin_right: 10, margin_top: 10, margin_bottom: 30,
    ///     x_axis: AxisConfig::new(0.0, 10.0, 2.0, b"Time"),
    ///     y_axis: AxisConfig::new(0.0, 100.0, 20.0, b"Value"),
    ///     bg_color: Color::BLACK,
    ///     line_color: Color::GREEN,
    ///     axis_color: Color::WHITE,
    ///     grid_color: Color::from_rgb(64, 64, 64),
    ///     text_color: Color::WHITE,
    ///     label_color: Color::CYAN,
    /// };
    ///
    /// let chart: LineChart<100> = LineChart::new(config);
    /// ```
    pub fn new(config: PlotConfig) -> Self {
        assert!(N <= PLOT_HISTORY_LIMIT, "L'historique dépasse la limite physique horizontale.");
        assert!(config.x_axis.is_valid(), "Configuration axe X invalide.");
        assert!(config.y_axis.is_valid(), "Configuration axe Y invalide.");
        
        let plot_x = config.x + config.margin_left;
        let plot_y = config.y + config.margin_top;
        let plot_w = config.width - config.margin_left - config.margin_right;
        let plot_h = config.height - config.margin_top - config.margin_bottom;

        Self {
            config,
            data: [0.0; N],
            head: 0,
            count: 0,
            plot_x,
            plot_y,
            plot_w,
            plot_h,
        }
    }

    /// Retourne une référence à la configuration du graphique.
    #[inline]
    pub fn config(&self) -> &PlotConfig {
        &self.config
    }

    /// Ajoute une nouvelle valeur à l'historique.
    ///
    /// Si le buffer est plein (N points), la valeur la plus ancienne est remplacée.
    ///
    /// # Arguments
    ///
    /// * `value` - Valeur à ajouter (sera clampée à la plage Y-axis lors du rendu)
    ///
    /// # Exemple
    ///
    /// ```
    /// use embassy_st7789v::{Color};
    /// use embassy_st7789v_plot::{AxisConfig, PlotConfig, LineChart};
    ///
    /// let config = PlotConfig {
    ///     x: 10, y: 10, width: 220, height: 200,
    ///     margin_left: 40, margin_right: 10, margin_top: 10, margin_bottom: 30,
    ///     x_axis: AxisConfig::new(0.0, 10.0, 2.0, b"Time"),
    ///     y_axis: AxisConfig::new(0.0, 100.0, 20.0, b"Value"),
    ///     bg_color: Color::BLACK,
    ///     line_color: Color::GREEN,
    ///     axis_color: Color::WHITE,
    ///     grid_color: Color::from_rgb(64, 64, 64),
    ///     text_color: Color::WHITE,
    ///     label_color: Color::CYAN,
    /// };
    ///
    /// let mut chart: LineChart<100> = LineChart::new(config);
    /// chart.push(50.0);
    /// chart.push(55.5);
    /// ```
    pub fn push(&mut self, value: f32) {
        self.data[self.head] = value;
        self.head = (self.head + 1) % N;
        if self.count < N {
            self.count += 1;
        }
    }

    /// Efface l'historique et réinitialise le graphique.
    ///
    /// # Exemple
    ///
    /// ```
    /// use embassy_st7789v::{Color};
    /// use embassy_st7789v_plot::{AxisConfig, PlotConfig, LineChart};
    ///
    /// let config = PlotConfig {
    ///     x: 10, y: 10, width: 220, height: 200,
    ///     margin_left: 40, margin_right: 10, margin_top: 10, margin_bottom: 30,
    ///     x_axis: AxisConfig::new(0.0, 10.0, 2.0, b"Time"),
    ///     y_axis: AxisConfig::new(0.0, 100.0, 20.0, b"Value"),
    ///     bg_color: Color::BLACK,
    ///     line_color: Color::GREEN,
    ///     axis_color: Color::WHITE,
    ///     grid_color: Color::from_rgb(64, 64, 64),
    ///     text_color: Color::WHITE,
    ///     label_color: Color::CYAN,
    /// };
    ///
    /// let mut chart: LineChart<100> = LineChart::new(config);
    /// chart.push(50.0);
    /// chart.clear();
    /// ```
    pub fn clear(&mut self) {
        self.head = 0;
        self.count = 0;
        self.data.fill(0.0);
    }

    #[inline]
    fn get_sample(&self, index: usize) -> f32 {
        let oldest = if self.count < N { 0 } else { self.head };
        self.data[(oldest + index) % N]
    }

    /// Convertit une valeur Y en coordonnée écran (pixels).
    ///
    /// La valeur est clampée à la plage [y_min, y_max] définie par `y_axis`,
    /// puis convertie linéairement en pixels.
    #[inline]
    fn scale_y(&self, value: f32) -> i32 {
        let y_min = self.config.y_axis.start;
        let y_max = self.config.y_axis.end;
        
        if y_max <= y_min {
            return self.plot_y + self.plot_h - 1;
        }

        let clamped_val = value.max(y_min).min(y_max);
        let ratio = (clamped_val - y_min) / (y_max - y_min);
        
        self.plot_y + self.plot_h - 1 - (ratio * (self.plot_h - 1) as f32) as i32
    }

    /// Convertit un index de données en coordonnée X écran (pixels).
    ///
    /// Les N points sont distribués uniformément sur la largeur de la zone de tracé.
    #[inline]
    fn scale_x(&self, index: usize) -> i32 {
        if N <= 1 {
            return self.plot_x;
        }
        self.plot_x + (index as i32 * (self.plot_w - 1)) / (N as i32 - 1)
    }

    /// Convertit une valeur d'axe X en coordonnée écran (pour labels).
    ///
    /// Similaire à `scale_y`, mais pour l'axe X.
    #[inline]
    fn scale_x_value(&self, value: f32) -> i32 {
        let x_min = self.config.x_axis.start;
        let x_max = self.config.x_axis.end;
        
        if x_max <= x_min {
            return self.plot_x;
        }
        
        let clamped = value.max(x_min).min(x_max);
        let ratio = (clamped - x_min) / (x_max - x_min);
        self.plot_x + (ratio * (self.plot_w - 1) as f32) as i32
    }

    /// Affiche le graphique complet avec grille, axes, graduations et courbe.
    ///
    /// Cette méthode effectue :
    /// 1. Remplissage du fond
    /// 2. Grille horizontale (Y) et labels des graduations
    /// 3. Grille verticale (X) et labels des graduations
    /// 4. Labels des axes (titres)
    /// 5. Bordures externes
    /// 6. Tracé de la courbe (données)
    ///
    /// # Arguments
    ///
    /// * `gfx` - Contexte graphique initialisé
    ///
    /// # Exemple
    ///
    /// ```no_run
    /// # use embassy_st7789v::{Color, St7789v, NoPin};
    /// # use embedded_hal::digital::OutputPin;
    /// # use embedded_hal_async::spi::SpiDevice;
    /// use embassy_st7789v_plot::{Graphics, AxisConfig, PlotConfig, LineChart};
    ///
    /// # async fn example<SPI, DC>(display: &mut St7789v<SPI, DC, NoPin>) where SPI: SpiDevice, DC: OutputPin {
    /// let config = PlotConfig {
    ///     x: 10, y: 10, width: 220, height: 200,
    ///     margin_left: 40, margin_right: 10, margin_top: 10, margin_bottom: 30,
    ///     x_axis: AxisConfig::new(0.0, 10.0, 2.0, b"Time"),
    ///     y_axis: AxisConfig::new(0.0, 100.0, 20.0, b"Value"),
    ///     bg_color: Color::BLACK,
    ///     line_color: Color::GREEN,
    ///     axis_color: Color::WHITE,
    ///     grid_color: Color::from_rgb(64, 64, 64),
    ///     text_color: Color::WHITE,
    ///     label_color: Color::CYAN,
    /// };
    ///
    /// let mut chart: LineChart<100> = LineChart::new(config);
    /// for i in 0..10 {
    ///     chart.push((i as f32) * 10.0);
    /// }
    ///
    /// let mut gfx = Graphics::new_no_rst(display);
    /// chart.render(&mut gfx).await;
    /// # }
    /// ```
    pub async fn render<SPI, DC, RST>(&self, gfx: &mut Graphics<'_, SPI, DC, RST>)
    where
        SPI: SpiDevice,
        DC: OutputPin,
        RST: OutputPin,
    {
        // 1. Fond de la zone du graphique
        let _ = gfx.display.fill_rect(
            self.plot_x as u16,
            self.plot_y as u16,
            (self.plot_x + self.plot_w - 1) as u16,
            (self.plot_y + self.plot_h - 1) as u16,
            self.config.bg_color,
        ).await;

        let right_edge = self.plot_x + self.plot_w - 1;
        let bottom_edge = self.plot_y + self.plot_h - 1;

        // 2. Grille horizontale (Y) + Labels Y
        let y_axis = &self.config.y_axis;
        let y_range = y_axis.end - y_axis.start;
        let tick_count_y = y_axis.tick_count();

        for i in 0..tick_count_y {
            let value = y_axis.start + (i as f32 * y_axis.step);
            let ratio = (value - y_axis.start) / y_range;
            let y_grid = bottom_edge - (ratio * (self.plot_h - 1) as f32) as i32;

            // Ligne de grille (sauf sur les bordures)
            if i > 0 && i < tick_count_y - 1 {
                let _ = gfx.display.draw_hline(
                    self.plot_x as u16, 
                    y_grid as u16, 
                    self.plot_w as u16, 
                    self.config.grid_color
                ).await;
            }

            // Label Y dans la marge gauche (avec padding pour éviter la coupure)
            let label_color = if value.abs() < 0.01 * y_axis.step {
                Color::GREEN  // Met en évidence la valeur zéro
            } else {
                self.config.text_color
            };

            let label_y = (y_grid - 4).max(self.config.y + 8).min(self.config.y + self.config.height - 8);

            let _ = gfx.display.draw_f32(
                (self.config.x + 2) as u16,
                label_y as u16,
                value,
                1,
                label_color,
                self.config.bg_color,
            ).await;
        }

        // 3. Grille verticale (X) + Labels X
        let x_axis = &self.config.x_axis;
        
        let tick_count_x = x_axis.tick_count();

        for i in 0..tick_count_x {
            let value = x_axis.start + (i as f32 * x_axis.step);
            let x_grid = self.scale_x_value(value);

            // Ligne de grille (sauf sur les bordures)
            if i > 0 && i < tick_count_x - 1 {
                let _ = gfx.display.draw_vline(
                    x_grid as u16,
                    self.plot_y as u16,
                    self.plot_h as u16,
                    self.config.grid_color
                ).await;
            }

            // Label X dans la marge inférieure (avec padding pour éviter la coupure)
            let label_x = (x_grid - 8).max(self.config.x + 2).min(self.config.x + self.config.width - 20);

            let _ = gfx.display.draw_f32(
                label_x as u16,
                (bottom_edge + 4) as u16,
                value,
                1,
                self.config.text_color,
                self.config.bg_color,
            ).await;
        }

        // 4. Labels des axes (titres) — draw_str prend &[u8] et 5 arguments
        // Label Y (coin haut-gauche de la marge, 2 lignes plus haut)
        let _ = gfx.display.draw_str(
            (self.config.x + 2) as u16,
            (self.config.y - 16).max(0) as u16,
            y_axis.label,
            self.config.label_color,
            self.config.bg_color,
        ).await;

        // Label X (dans la marge bas, 2 lignes plus bas)
        let label_x_x = self.plot_x + (self.plot_w / 2) - ((x_axis.label.len() as i32 * 6) / 2);
        let _ = gfx.display.draw_str(
            label_x_x.max(self.config.x + 2) as u16,
            //Le label X est placé dans la marge inférieure, avec un padding de 4 pixels pour éviter la coupure
            (self.config.y + self.config.height + 4).min(SCREEN_H as i32 - 8) as u16, 
            x_axis.label,
            self.config.label_color,
            self.config.bg_color,
        ).await;

        // 5. Bordures externes
        let _ = gfx.display.draw_hline(self.plot_x as u16, self.plot_y as u16, self.plot_w as u16, self.config.axis_color).await;
        let _ = gfx.display.draw_hline(self.plot_x as u16, bottom_edge as u16, self.plot_w as u16, self.config.axis_color).await;
        let _ = gfx.display.draw_vline(self.plot_x as u16, self.plot_y as u16, self.plot_h as u16, self.config.axis_color).await;
        let _ = gfx.display.draw_vline(right_edge as u16, self.plot_y as u16, self.plot_h as u16, self.config.axis_color).await;

        // 6. Tracé des données
        if self.count < 2 {
            if self.count == 1 {
                let px = self.scale_x(0);
                let py = self.scale_y(self.get_sample(0));
                gfx.pixel(px, py, self.config.line_color).await;
            }
            return;
        }

        let mut prev_x = self.scale_x(0);
        let mut prev_y = self.scale_y(self.get_sample(0));

        for i in 1..self.count {
            let next_x = self.scale_x(i);
            let next_y = self.scale_y(self.get_sample(i));

            line(gfx, prev_x, prev_y, next_x, next_y, self.config.line_color).await;

            prev_x = next_x;
            prev_y = next_y;
        }
    }
}