package anonymousgeneric
fun ints(base: Int): Base<Int> = object : Base<Int>(base) {
    override fun apply(value: Int): Int = super.apply(value) + base
}
fun strings(base: String): Base<String> = object : Middle<String>(base), Mapper<String> {
    override fun apply(value: String): String = super.apply(value) + base
    override fun map(value: String): String = value + base
}
fun nullable(): Mapper<String?> = object : Mapper<String?> {
    override fun map(value: String?): String? = value
}
fun longMapper(): Mapper<Long> = object : Mapper<Long> {
    override fun map(value: Long): Long = value + 7L
}
fun listMapper(): Mapper<List<String>> = object : Mapper<List<String>> {
    override fun map(value: List<String>): List<String> = value
}
fun boxedMapper(): Mapper<Int?> = object : Mapper<Int?> {
    override fun map(value: Int?): Int? = value
}
