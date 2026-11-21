class Good { fun inc(): Good = this }
fun use() { var value = Good(); value++ }
