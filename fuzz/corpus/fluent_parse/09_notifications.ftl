notifications =
    { $count ->
        [0] No new notifications
        [1] One new notification
       *[other] { $count } new notifications
    }
    .tooltip =
        { $count ->
            [0] You're all caught up!
           *[other] Check your updates
        }