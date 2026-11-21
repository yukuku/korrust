class Good { operator fun inc(): Good = this }
fun use(value: Good) { value++ }
