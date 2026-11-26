class Holder(private val token: Token?) {
    fun read(): Int { if (token != null) return token.value(); return -1 }
}
