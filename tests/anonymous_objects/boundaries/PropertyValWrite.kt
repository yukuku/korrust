interface A { fun n(): Int }
fun bad(): A = object : A {
    val state: Int = 1
    override fun n(): Int { state = 2; return state }
}
