fun consumer(): String {
    val result: inherited.library.JavaTypes.Base.Result = inherited.consumer.Direct().bodyOnly()
    val token: inherited.library.JavaTypes.Base.Token<String> = inherited.consumer.Direct().token()
    val own: inherited.consumer.Own.Result = inherited.consumer.Own().make()
    return result.tag() + token.value + own.value
}

suspend fun consumerSuspend(): inherited.library.JavaTypes.Base.Result = inherited.consumer.SuspendEnd().execute()
