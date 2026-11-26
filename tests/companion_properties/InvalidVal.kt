package companionstate
class ReadOnly {
    fun invalid() { fixed = 3 }
    companion object { val fixed: Int = 1 }
}
