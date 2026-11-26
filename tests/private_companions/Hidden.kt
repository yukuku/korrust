package companionhidden

class Vault {
    fun read(): String = TOKEN
    fun qualified(): String = Vault.TOKEN
    fun explicit(): String = Companion.TOKEN
    fun called(): String = Companion.helper()
    fun property(): String = Companion.message
    fun shadow(Companion: String): String = Companion
    private companion object {
        fun helper(): String = TOKEN
        val message: String = "private-companion-ok"
        const val TOKEN: String = "private-companion-ok"
    }
}

class Empty {
    private companion object
}
