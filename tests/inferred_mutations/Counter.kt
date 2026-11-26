class InferredCounter(var current: Int) {
    var setterCalls: Int = 0
    private var stored = false
        set(value) {
            field = value
            setterCalls++
        }
    fun update(flag: Boolean) = when {
        flag -> { stored = true; current += 2; current }
        else -> { stored = false; current -= 1; current }
    }
    fun isStored() = stored
}
