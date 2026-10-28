fun box(): String {
    Box("OK").run { outer ->
        return outer
    }

    return "Fail"
}