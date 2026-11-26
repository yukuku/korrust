package companionstate
class WrongType {
    fun invalid() { text = 3 }
    companion object { var text: String? = null }
}
