package memberfunctionlibrary
object Callbacks {
    var reads: Int = 0
    @JvmStatic val action: (Int) -> Int
        get() { reads += 1; return { it + 3 } }
    @JvmStatic val amount: Int get() = 42
}
