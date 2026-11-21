package boundary
import inference.dependency.exists
fun captured(value: String?): () -> Int = {
    if (exists(value)) value.length else -1
}
