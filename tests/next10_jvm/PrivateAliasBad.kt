package nextjvm.consumer
import nextjvm.api.Secret
typealias Alias = Secret
fun invalid(): Secret = Alias(1)
