package companionstate
class PrivateCapture {
    fun callback(): () -> Int = { Companion.answer() }
    private companion object { fun answer(): Int = 7 }
}
