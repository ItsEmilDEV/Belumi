# Design: Belumi launcher

Mode: Operate. Surface: a 760x500 native window (egui). Three styles share one layout (left navigation 170 px, content, status bar) and differ in material.

## Tokens

| Token | Modern | Modern Glass | Pixel |
|---|---|---|---|
| bg | #0E1015 | gradient #14102A to #0B1220 | #F7E2AB parchment with flecks, framed in a tan board with 2 px #5A2E12 outlines |
| panel (nav, status) | #14171E | #FFFFFF at 6% over bg | #EFD391 |
| nav / status bar | #14171E | #FFFFFF at 12% / 10% | wooden table #D7A463 with seams; tabs are tan buttons, the active one cream with a green marker |
| card | #1B1F29 | #FFFFFF at 8%, 1 px #FFFFFF at 14% edge | #FFF1C9 inset, notched corners |
| card hover | #242936 | #FFFFFF at 12% | #F3CD8C |
| text | #E6E8EE | #F0EEFA | #3B1D0A |
| muted | #9AA1B2 | #B4B0CC | #5A381B |
| accent | #9B5CFF | #A981FF | #3F8A22 (leaf green) |
| accent dim | #2A1B52 | #A981FF at 22% | #BDDB82 |
| on accent | #FFFFFF | #FFFFFF | #FFFFFF with 1 px #1B3D0C shadow |
| ok / warn / danger | #4ADE80 / #FBBF24 / #F87171 | same | #2F6E17 / #8A4F00 / #8A1414 |
| radius (card / control) | 12 / 8 | 16 / 10 | 0 / 0 |
| border | 1 px #242833 | 1 px #FFFFFF at 14% | 2 px #5A2E12 |
| bevel | none | none | 2 px white-at-47% lip top-left, black-at-20% lip bottom-right, corners notched 2 px |

The accent follows the logo's violet in the two modern styles. Pixel is a warm, cosy farm-sim look in the spirit of Stardew Valley (own art, no game assets): dark text on parchment, tan buttons, green for the one primary action.

## Type

Proportional UI face (egui default). Scale: page title 22, card title 16 strong, body 14, muted 12.5, play button 16 strong. The Pixel style sets everything in the bundled pixel face (Pixelify Sans). Numerals in the log are monospace.

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
