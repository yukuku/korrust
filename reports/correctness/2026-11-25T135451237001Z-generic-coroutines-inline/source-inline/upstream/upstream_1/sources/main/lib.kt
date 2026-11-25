inline fun<reified T> isinstance(x: Any?): Boolean {
    return x is T
}

