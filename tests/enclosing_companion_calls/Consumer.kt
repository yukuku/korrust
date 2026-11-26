class Shell {
    companion object {
        @JvmStatic fun evaluate(value: Int): Int = value + 2
        fun tag(value: String): String = value
    }
    class Middle {
        companion object { fun evaluate(value: String): Int = value.length }
        class Inner {
            fun score(): Int = evaluate(3) * 10 + evaluate(tag("abc"))
        }
    }
}
