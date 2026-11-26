interface A { val n: Int }
fun bad(): A = object : A {
    override val n: Int = 1
        get() { field = 2; return field }
}
