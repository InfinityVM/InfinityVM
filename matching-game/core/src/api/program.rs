//! Types for the program and contract APIs.

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
