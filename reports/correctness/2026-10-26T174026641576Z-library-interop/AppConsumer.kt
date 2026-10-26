package interop.appconsumer
import yuku.alkitab.util.IntArrayList
import yuku.alkitab.base.util.toIntArray
fun main() { val value = IntArrayList(0); value.add(17); check(value.toIntArray().contentEquals(intArrayOf(17))); println("app consumer passed") }
