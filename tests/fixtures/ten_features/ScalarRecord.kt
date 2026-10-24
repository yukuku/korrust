package ten

data class ScalarRecord(
    val count: Int,
    val tiny: Byte,
    val character: Char,
    val isReady: Boolean,
    val timestamp: Long,
    val message: String,
)
