//! Shared logic and types of the matching game.

alloy::sol! {
    /// A pair of users that matched.
    #[derive(Default, PartialEq, Eq, PartialOrd, Ord, Debug)]
    struct Match {
        /// First user in pair.
        address user1;
        /// Second user in pair.
        address user2;
    }
}

/// Array of matches. Result from matching game program.
pub type Matches = alloy::sol! {
    Match[]
};

/// Matching game server API types.
pub mod api;
