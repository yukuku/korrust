package ten

class PublicFields(
    val count: Int,
    val timestamp: Long,
    val label: String,
    val isReady: Boolean,
    val isTitle: String,
    val optional: String?,
    val texts: List<String?>,
    private val hidden: Int,
)
