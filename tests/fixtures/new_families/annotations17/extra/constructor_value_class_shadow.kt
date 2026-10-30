package annotation17extra
import annotation17.*
class Bad(Token: Int) { @Rich(type = Token::class) fun bad(): Int = 1 }
