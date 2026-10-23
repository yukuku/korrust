package korrust.fixtures

interface IndexedText {
    val itemCount: Int
    fun itemAt(position: Int): String
    fun label(position: Int): String = "slot:" + position
}
