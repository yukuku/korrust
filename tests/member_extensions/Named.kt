class Named {
    companion object Factory {
        fun String.decorate(): String = this + "K"
        val String.decorated: String get() = this + "K"
        fun ordinary(): String = "OK"
    }
}
