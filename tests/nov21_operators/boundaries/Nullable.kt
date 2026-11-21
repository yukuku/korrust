class Good { operator fun inc(): Good = this }
fun use(initial: Good?) { var value = initial; value++ }
