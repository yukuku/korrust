fun critical(lock: Any): Boolean = synchronized(lock) { Thread.holdsLock(lock) }
fun call(lock: Any, action: () -> Int): Int = synchronized(lock) { action() }
fun abrupt(lock: Any): Int = synchronized<Int>(lock) { throw IllegalStateException("failure") }
fun early(lock: Any): Int {
    synchronized(lock) { if (Thread.holdsLock(lock)) return 7; 0 }
    return 0
}
fun nested(outer: Any, inner: Any): Boolean = synchronized(outer) {
    synchronized(inner) { Thread.holdsLock(outer) && Thread.holdsLock(inner) }
}
fun tryOutside(lock: Any): Int {
    try { return abrupt(lock) } catch (failure: IllegalStateException) { return 9 }
}
