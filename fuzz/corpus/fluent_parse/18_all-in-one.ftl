all-in-one =
    { $username }, you have { $emailCount ->
        [0] no new emails
        [1] one new email
       *[other] { $emailCount } new emails
    } as of { DATETIME($lastChecked) }.
    .tooltip = Last checked: { DATETIME($lastChecked, hour: "2-digit", minute: "2-digit") }