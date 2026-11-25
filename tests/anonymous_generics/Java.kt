package anonymousgeneric
fun javaStrings(base: String): JavaBase<String> = object : JavaBase<String>(base), Mapper<String> {
    override fun apply(value: String): String = super.apply(value) + base
    override fun map(value: String): String = value + base
}
fun javaNullable(base: String?): JavaBase<String> = object : JavaBase<String>(base) {
    override fun apply(value: String?): String? = value
}
