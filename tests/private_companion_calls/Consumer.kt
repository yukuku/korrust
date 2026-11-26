class Factory {
    companion object {
        private fun secret(n: Int): Int = n + 7
    }
    fun value(): Int = secret(5)
    class Nested { fun value(): Int = secret(4) }
}
