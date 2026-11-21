interface Parent { val value: String }
class Bad(@JvmField override val value: String) : Parent
