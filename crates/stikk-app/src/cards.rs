//! **The overlay stack, and every surface the program hands out** (RFC 067 decision 4; Handoff C
//! §1).
//!
//! # Why this is a type and not three fields
//!
//! A card on top must never inherit the reading of the card that was there before it. Until 0.22.1
//! that rule was kept by discipline: `push_overlay` and `pop_overlay` minted a surface, and the
//! fifty-three places that removed a card each minted on the line above. **It held at every push,
//! pop and remove, and it was never applied to a replace** — twenty-six places answered a preview
//! by writing a new card over the `Overlay::Loading` in its slot, with no mint. The Loading card is
//! short, so its first draw reports *"the end is on screen"* and the latch sets; the confirmation
//! that replaced it was then born already read, and **one `Enter` committed a card nobody had
//! read** (the 0.22.0 regression this module exists to make unwriteable).
//!
//! So the stack and the surface are behind a type whose fields `app.rs` cannot reach. There is no
//! way to put a card on top without going through [`Cards::push`], [`Cards::replace`],
//! [`Cards::remove`], [`Cards::pop`] or [`Cards::truncate`], and each of those mints when the top
//! changes. [`Cards::top_mut`] exists for editing the card that is already there — its `error`, its
//! `typed`, its `acknowledged` — and assigning a whole card through it is the one bypass left, which
//! is why its doc says so and why §4's tests cover every replace site by behaviour.
//!
//! # One minter for every surface
//!
//! [`Cards`] also mints the surfaces of **screens** ([`Cards::mint_for_screen`]). That is not tidiness:
//! `stikk-tui` keys its scroll offsets by [`Surface`], so an overlay and a screen that were minted
//! from two separate counters could collide and share one reader's place in two different texts.
//! One counter cannot.

use crate::app::Surface;
use crate::overlay::Overlay;

/// The overlay stack and the surface of the card on top.
#[derive(Debug)]
pub(crate) struct Cards {
    /// The stack; the last is the card that draws and takes keys.
    stack: Vec<Overlay>,
    /// The surface of the card on top. Re-minted whenever that card changes.
    top: Surface,
    /// The next surface to hand out. Never reused, so a late report can always be told apart.
    next: u64,
}

impl Default for Cards {
    fn default() -> Self {
        Self {
            stack: Vec::new(),
            top: Surface::first(),
            next: 1,
        }
    }
}

impl Cards {
    /// A surface no earlier card or screen can be mistaken for.
    fn mint(&mut self) -> Surface {
        let surface = Surface::of(self.next);
        self.next = self.next.wrapping_add(1);
        surface
    }

    /// **A surface for a screen**, from the one counter every surface comes from (see the module
    /// doc: `stikk-tui` keys its offsets by surface, so two counters could collide).
    pub(crate) fn mint_for_screen(&mut self) -> Surface {
        self.mint()
    }

    /// The surface of the card on top. Meaningless when the stack is empty, and
    /// `App::reading_surface` asks [`Self::is_empty`] first.
    pub(crate) fn surface(&self) -> Surface {
        self.top
    }

    /// Put a card on top. **It gets a reader of its own.**
    pub(crate) fn push(&mut self, overlay: Overlay) {
        self.top = self.mint();
        self.stack.push(overlay);
    }

    /// Take the top card off. Whatever is under it is a different card, so it gets a new surface
    /// rather than the reading it had before it was covered.
    pub(crate) fn pop(&mut self) -> Option<Overlay> {
        self.top = self.mint();
        self.stack.pop()
    }

    /// **Write a different card into `index`'s slot** — how a preview's answer replaces the
    /// `Overlay::Loading` that was waiting for it.
    ///
    /// **Mints only when `index` is the top** (trap 1). A card replaced below the top changes
    /// nothing the reader is reading, and minting then would throw away the latch of the card they
    /// *are* reading; [`Self::pop`] mints when they come back down to it.
    pub(crate) fn replace(&mut self, index: usize, overlay: Overlay) {
        let Some(slot) = self.stack.get_mut(index) else {
            return;
        };
        *slot = overlay;
        if index + 1 == self.stack.len() {
            self.top = self.mint();
        }
    }

    /// Take the card at `index` out, wherever it is.
    ///
    /// **Always mints**, which is what the fifty-three hand-written lines this replaced did. A
    /// removal below the top therefore costs the reader the top card's latch — it errs closed, and
    /// that is the behaviour 0.21.1 and 0.22.0 both had.
    pub(crate) fn remove(&mut self, index: usize) -> Option<Overlay> {
        if index >= self.stack.len() {
            return None;
        }
        self.top = self.mint();
        Some(self.stack.remove(index))
    }

    /// Drop every card from `len` upwards (how a stale flow is torn down).
    pub(crate) fn truncate(&mut self, len: usize) {
        if len < self.stack.len() {
            self.top = self.mint();
        }
        self.stack.truncate(len);
    }

    /// The card on top.
    pub(crate) fn top(&self) -> Option<&Overlay> {
        self.stack.last()
    }

    /// **The card on top, to edit in place** — its `error`, its `typed`, its `acknowledged`.
    ///
    /// **Never to assign a whole card through.** `*top = Overlay::…` would be a replacement with no
    /// mint, which is the defect this module exists to prevent; use [`Self::replace`]. Every site
    /// that uses this is listed in Handoff C's request, with which kind it is.
    pub(crate) fn top_mut(&mut self) -> Option<&mut Overlay> {
        self.stack.last_mut()
    }

    /// Whether no card is open.
    pub(crate) fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// How many cards are open — for the tests that count what a reload left on the stack.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.stack.len()
    }

    /// The stack, oldest first, for a search over what is open.
    pub(crate) fn iter(&self) -> std::slice::Iter<'_, Overlay> {
        self.stack.iter()
    }
}

#[cfg(test)]
mod tests;
