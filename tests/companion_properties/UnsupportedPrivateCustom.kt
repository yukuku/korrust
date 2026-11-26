package companionstate
class CustomPrivate {
    fun update() { count = 3 }
    companion object {
        private var count: Int = 0
            set(value) { field = value * 2 }
    }
}
