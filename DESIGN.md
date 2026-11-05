# Design: Belumi launcher

Mode: Operate. Surface: a 760x500 native window (egui). Three styles share one layout (left navigation 170 px, content, status bar) and differ in material.

## Tokens

| Token | Modern | Modern Glass | Minecraft |
|---|---|---|---|
| bg | #0E1015 | gradient #14102A to #0B1220 | #3B3B3B stone, 16 px tile pattern |
| panel (nav, status) | #14171E | #FFFFFF at 6% over bg | #2B2B2B |
| nav / status bar | #14171E | #FFFFFF at 12% / 10% | planks #6F5230 with grain / dirt #4A3622 under a grass strip |
| card | #1B1F29 | #FFFFFF at 8%, 1 px #FFFFFF at 14% edge | #4A4A4A with bevel |
| card hover | #242936 | #FFFFFF at 12% | #5A5A5A |
| text | #E6E8EE | #F0EEFA | #F2F2F2 |
| muted | #9AA1B2 | #B4B0CC | #C0C0C0 |
| accent | #9B5CFF | #A981FF | #5DAA2F (grass green) |
| accent dim | #2A1B52 | #A981FF at 22% | #3A6B1C |
| on accent | #FFFFFF | #FFFFFF | #FFFFFF with 1 px #1D3A0C shadow |
| ok / warn / danger | #4ADE80 / #FBBF24 / #F87171 | same | #7CD64B / #FFD13A / #E5503A |
| radius (card / control) | 12 / 8 | 16 / 10 | 0 / 0 |
| border | 1 px #242833 | 1 px #FFFFFF at 14% | 2 px #101010 |
| bevel | none | none | light #8A8A8A top-left, dark #1E1E1E bottom-right, 2 px |

The accent follows the logo's violet in the two modern styles. Minecraft uses green so the play button reads as the game's own.

## Type

Proportional UI face (egui default). Scale: page title 22, card title 16 strong, body 14, muted 12.5, play button 16 strong. Minecraft style sets titles and buttons in the monospace face with no italics. Numerals in the log are monospace.

## Spacing

4 / 8 / 12 / 16 / 20. Cards pad 16, pages pad 20, items 10 apart. More space above a page title than below.

## Components

- Navigation: four text rows, the active one filled with accent dim; no icons.
- Card: one level only, never nested; account card shows the skin head (8x8 pixels scaled with nearest filtering), name, and one line of state.
- Primary button (Play): accent fill, 42 px high, 170 px wide minimum. Secondary buttons: card fill. Disabled: 45% alpha with a hover reason.
- Status bar: a dot with a word (Idle, Working, Running) and the last message.
- Focus: 2 px accent outline in all styles.

## States

Hover lifts the fill one step; pressed drops it; disabled keeps the label readable but dims; busy shows the device code in a card with a large monospace code; errors are a sentence in the status bar plus danger-coloured text on the page they belong to.

## Do not

Gradient text, side-stripe borders, nested cards, glass on text-heavy areas (the log stays solid in every style), icons drawn from glyphs.

## Type faces

Modern and Glass set everything in Inter (bundled, OFL). Minecraft sets everything in Pixelify Sans (bundled, OFL), titles with a 2 px black offset shadow. Both fall back to egui's default faces for glyphs they lack.

## Layout (820 x 600, minimum 700 x 520)

Navigation 176 px with drawn icons; content padded 22 / 18; status bar 34 px (40 with the Minecraft grass strip). The Play page reads who / go / where from top to bottom: account card, a 240 x 56 Play button with two quiet text actions beside it, one muted sentence saying what will start, then a segmented Servers / Worlds list whose rows are 46 px high.
