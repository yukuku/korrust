fun perform(left: Boolean, fail: Boolean) {
    Effects.accept {
        try {
            if (fail) throw IllegalStateException()
            if (left) Effects.left() else Effects.right()
        } catch (e: IllegalStateException) {
            if (left) Effects.right() else Effects.left()
        } finally { Effects.right() }
    }
}
fun direct(left: Boolean) {
    try { if (left) Effects.left() else Effects.right() } finally { Effects.right() }
}
