# Changelog

Tous les changements notables de ce projet seront documentés dans ce fichier.

Le format est basé sur [Keep a Changelog](https://keepachangelog.com/fr/1.0.0/) et ce projet respecte le [Semantic Versioning](https://semver.org/lang/fr/).

## [0.2.0] - 2026-05-29

### Ajouté
- **Rendu intelligent (Double Phase)** : Séparation stricte entre le dessin d'initialisation (statique) et le rafraîchissement dynamique de la grille et des courbes pour maximiser les performances d'affichage.
- **Détection automatique du zéro** : Surlignage visuel spécifique en vert (ou couleur configurée) lorsque la valeur `0.0` croise les axes de la grille.
- **Protection stricte des bordures** : Clamping géométrique systématique de toutes les primitives de dessin à l'espace interne utile pour éviter toute bavure ou débordement sur les étiquettes textuelles.

### Modifié
- Optimisation de la structure de stockage interne du ring buffer afin de garantir zéro allocation sur la stack lors des opérations d'empilement asynchrones.

---

## [0.1.1] - 2026-05-27

### Corrigé
- **Doc comment invalide dans `line()`** : Remplacement du doc comment `//!` (réservé à la documentation de niveau module) par un doc string `///` placé immédiatement avant la déclaration de la fonction, conformément aux règles strictes du compilateur Rust.
- **Calcul de centrage du label X** : Correction de l'indexation de champ ; utilisation de `x_axis.label.len()` au lieu de `y_axis.label.len()` pour calculer la position horizontale centrée du titre de l'axe X.
- **Protection contre le débordement du label X** : Ajout d'une opération de clamping `.min(SCREEN_H as i32 - 8)` empêchant le label textuel de l'axe X de déborder de la hauteur physique maximale de l'écran ST7789V.