package use
import external.Parent
class Host : Parent() {
    fun run(): String = format(2, 5)
    companion object {
        fun format(a: String): String = format(a, 4)
        fun pick(): String = format("one")
    }
}
