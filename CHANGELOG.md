# Changelog
Tous les changements notables de ce projet seront documentés dans ce fichier.

[0.1.1] - 2026-05-27

## Corrigé

   - 1. Doc comment invalide dans line() :Remplacement du doc comment //! (réservé aux modules) par /// placé avant la fonction, conformément aux règles Rust sur les doc strings.

   - 2. Calcul du centrage du label X : Utilisation de x_axis.label.len() au lieu de y_axis.label.len() pour le calcul de la position centrée du titre de l'axe X.

   - 3. Protection contre le débordement d'écran du label X :Ajout d'un clamp .min(SCREEN_H as i32 - 8) pour empêcher le label de l'axe X de dépasser la hauteur physique de l'écran ST7789V

