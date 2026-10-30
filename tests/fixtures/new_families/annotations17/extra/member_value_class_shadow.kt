package annotation17extra
import annotation17.*
class Bad { val Token: Int = 1
@Rich(type = Token::class) fun bad(): Int = 1 }
