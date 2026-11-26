package receiverlib

inline fun <T> T.configureValue(block: T.() -> Unit): T {
    block()
    return this
}

inline fun <T, R> T.inspectValue(block: T.() -> R): R = block()
