fun consume(lock: Any): Boolean {
    return critical(lock) && call(lock) { 8 } == 8 && early(lock) == 7 && nested(lock, lock) && tryOutside(lock) == 9 && !Thread.holdsLock(lock)
}
