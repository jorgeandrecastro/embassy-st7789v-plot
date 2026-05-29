# embassy-st7789v-plot

Moteur de tracé de graphiques cartésiens (X, Y) adaptatifs et configurables pour écrans TFT LCD **ST7789V 240×320**, construit au-dessus de [`embassy-st7789v`](https://crates.io/crates/embassy-st7789v).

## 🎯 Caractéristiques

- ✅ **`#![no_std]` + `#![forbid(unsafe_code)]`** : Entièrement sûr et embarqué
- ✅ **Zéro allocation dynamique** : Buffers statiques uniquement (ring buffer fixe)
- ✅ **Historique circulaire** : Jusqu'à 240 points (limite physique écran 240px)
- ✅ **Axes configurables** : Graduations statiques avec pas personnalisable
- ✅ **Grille configurable** : Grille horizontale/verticale avec labels personnalisés
- ✅ **API asynchrone** : Intégration complète Embassy pour non-bloquant
- ✅ **Détection automatique du zéro** : La ligne zéro est surlignée en vert quand elle traverse la grille
- ✅ **Protection stricte des bordures** : Clamping des primitives géométriques à l'espace interne utile
- ✅ **Rendu ligne Bresenham** : Courbes lisses entre points de données
- ✅ **Rendu intelligent (Double Phase)** : Le cadre, les étiquettes et les titres ne sont tracés qu'une seule fois à l'initialisation. Seuls la grille interne et le signal sont rafraîchis dynamiquement.

-----

## Changelog

Voir [CHANGELOG.md](./CHANGELOG.md) pour l'historique des versions et modifications.

----

## 📦 Installation

```toml
[dependencies]
embassy-st7789v-plot = "0.2.0"
embassy-st7789v = "0.1"
embedded-hal = "1.0"
embedded-hal-async = "1.0"
```

----

## 🚀 Utilisation rapide

### Exemple basique : Single plot

```rust
use embassy_st7789v::{Color, St7789v, NoPin};
use embassy_st7789v_plot::{Graphics, AxisConfig, PlotConfig, LineChart};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Initialiser le display
    let mut display: St7789v<_, _, NoPin> = /* ... */;

    // Créer les axes
    let x_axis = AxisConfig::new(0.0, 10.0, 2.0, b"Time (s)");
    let y_axis = AxisConfig::new(0.0, 100.0, 20.0, b"Temperature (C)");

    // Configurer le graphique
    let config = PlotConfig {
        x: 10,
        y: 10,
        width: 220,
        height: 200,
        margin_left: 40,      // Pour labels Y
        margin_right: 10,
        margin_top: 10,
        margin_bottom: 30,    // Pour labels X
        x_axis,
        y_axis,
        bg_color: Color::BLACK,
        line_color: Color::GREEN,
        axis_color: Color::WHITE,
        grid_color: Color::from_rgb(64, 64, 64),
        text_color: Color::WHITE,
        label_color: Color::CYAN,
    };

    // Créer le gestionnaire de graphique (100 points max)
    let mut chart: LineChart<100> = LineChart::new(config);

    // Ajouter des données
    chart.push(45.2);
    chart.push(47.8);
    chart.push(52.1);
    chart.push(48.9);

    // Afficher
    let mut gfx = Graphics::new_no_rst(&mut display);
    chart.render(&mut gfx).await;
}
```

### Configuration des axes

```rust
// Axe X : temps de 0 à 60s, graduations tous les 10s
let time_axis = AxisConfig::new(0.0, 60.0, 10.0, b"Time (s)");

// Axe Y : température -10 à +50°C, graduations tous les 10°C
let temp_axis = AxisConfig::new(-10.0, 50.0, 10.0, b"Temp (C)");

// Axe Y : tensions 0 à 3.3V, graduations tous les 0.5V
let voltage_axis = AxisConfig::new(0.0, 3.3, 0.5, b"U (V)");

// Nombre de graduations affiché
assert_eq!(time_axis.tick_count(), 7);  // 0, 10, 20, 30, 40, 50, 60
```

### Validation de configuration

```rust
let axis = AxisConfig::new(0.0, 10.0, 0.5, b"X");
assert!(axis.is_valid());  // step > 0 && end > start

let invalid = AxisConfig::new(10.0, 0.0, 1.0, b"X");
assert!(!invalid.is_valid());  // end <= start
```

## 🎨 Schéma de positionnement

```
┌─────────────────────────────────┐  y
│ (x, y)                          │
│ ┌──────────────────────────┐    │
│ │  Label Y (amplitude)     │    │ margin_top
│ │  ┌────────────────────┐  │    │
│ │  │  100  ┼────────┐   │  │    │
│ │  │   80  │  •     │   │  │    │
│ │  │   60  │    •   │   │  │    │
│ │  │   40  │      •–┤   │  │    │
│ │  │   20  │        │   │  │    │
│ │  │    0  └────────┘   │  │    │
│ │  │       0  2  4  6  8 10     │ margin_bottom
│ │  │       Time (s)             │
│ │  └────────────────────────    │
│ └──────────────────────────────┘
  ↑                              ↑
margin_left              margin_right
```

----

## 📊 Structure de données

### Ring buffer (historique circulaire)

```
Stockage interne : array[100]
head = 3, count = 100 (buffer plein)

data[0] = valeur 97e
data[1] = valeur 98e
data[2] = valeur 99e  ← head (prochaine écriture)
data[3] = valeur 1ère  ← oldest (plus ancienne)
data[4] = valeur 2e
...
```

### Workflow de rendu (Double Phase)

**Phase d'initialisation (une seule fois):**
1. **Nettoyage global** → Effacement de l'espace d'affichage
2. **Labels Y** → Texte des graduations Y (vert si zéro détecté)
3. **Labels X** → Texte des graduations X
4. **Titres des axes** → Labels des axes X et Y
5. **Bordures fixes** → Cadre (axis_color)

**Phase dynamique (à chaque rendu):**
1. **Nettoyage interne** → Effacement strict de l'intérieur (sans toucher aux bordures)
2. **Grille Y** → Lignes horizontales (vert pour le zéro, couleur grille sinon)
3. **Grille X** → Lignes verticales
4. **Courbe** → Lignes Bresenham reliant les points de données

----

## 🔧 Cas d'usage

### Oscilloscope 1 canal

```rust
let config = PlotConfig {
    x: 0, y: 0, width: 240, height: 320,
    margin_left: 40, margin_right: 10,
    margin_top: 10, margin_bottom: 30,
    x_axis: AxisConfig::new(0.0, 240.0, 30.0, b"Samples"),
    y_axis: AxisConfig::new(-5.0, 5.0, 1.0, b"Voltage (V)"),
    // ... colors ...
};
let mut oscilloscope: LineChart<240> = LineChart::new(config);
```

### Moniteur de température

```rust
let config = PlotConfig {
    x: 10, y: 10, width: 220, height: 150,
    margin_left: 45, margin_right: 15,
    margin_top: 15, margin_bottom: 30,
    x_axis: AxisConfig::new(0.0, 120.0, 20.0, b"Time (min)"),
    y_axis: AxisConfig::new(15.0, 35.0, 5.0, b"T (°C)"),
    // ... colors ...
};
let mut temp_chart: LineChart<120> = LineChart::new(config);
```

### Graphique de pression

```rust
let config = PlotConfig {
    x: 5, y: 5, width: 230, height: 310,
    margin_left: 35, margin_right: 5,
    margin_top: 5, margin_bottom: 30,
    x_axis: AxisConfig::new(0.0, 1000.0, 100.0, b"Pa"),
    y_axis: AxisConfig::new(900.0, 1050.0, 30.0, b"P (hPa)"),
    // ... colors ...
};
let mut pressure_chart: LineChart<100> = LineChart::new(config);
```

----

## 🛠️ API complète

### `LineChart<N>`

| Méthode | Description |
|---------|-------------|
| `new(config)` | Crée un nouveau graphique |
| `push(value)` | Ajoute une valeur à l'historique |
| `clear()` | Efface l'historique |
| `config()` | Retourne la configuration |
| `render(gfx)` | Affiche le graphique (async) |

### `AxisConfig`

| Méthode | Description |
|---------|-------------|
| `new(start, end, step, label)` | Crée une config d'axe |
| `is_valid()` | Vérifie cohérence (step > 0 && end > start) |
| `tick_count()` | Nombre de graduations affiché |

### `Graphics<'a, SPI, DC, RST>`

| Méthode | Description |
|---------|-------------|
| `new(display)` | Crée contexte avec RST |
| `new_no_rst(display)` | Crée contexte sans RST |
| `pixel(x, y, color)` | Trace pixel (async) |

### Fonction globale

| Fonction | Description |
|----------|-------------|
| `line(gfx, x0, y0, x1, y1, color)` | Trace ligne Bresenham (async) |

----

## ⚠️ Limitations

- **Nombre de points** : Maximum 240 (largeur écran ST7789V)
- **Précision** : Axes en virgule flottante, pixel en entier
- **Pas non-adaptatif** : Les graduations sont statiques (pas de zoom ou rescaling automatique)
- **Labels** : Texte ASCII personnalisable pour les axes, valeurs flottantes pour les graduations

----

## 📝 Exemple complet avec boucle acquisition

```rust
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut display = /* init ST7789V */;
    
    let config = PlotConfig { /* ... */ };
    let mut chart: LineChart<200> = LineChart::new(config);
    let mut gfx = Graphics::new_no_rst(&mut display);

    // Boucle d'acquisition
    loop {
        // Lire capteur
        let value = sensor.read().await;
        chart.push(value);

        // Afficher tous les 100ms
        Timer::after(Duration::from_millis(100)).await;
        chart.render(&mut gfx).await;
    }
}
```

---

## 📜 License

GPL-2.0-or-later 

**Copyright (C) 2026 Jorge Andre Castro**

----

## 🔗 Dépendances

- [`embassy-st7789v`](https://crates.io/crates/embassy-st7789v) — Driver ST7789V
- [`embedded-hal`](https://crates.io/crates/embedded-hal) — Traits HAL
- [`embedded-hal-async`](https://crates.io/crates/embedded-hal-async) — Traits async HAL