# Como contribuir com o Lamina

Obrigado por querer ajudar! O Lamina é um projeto acadêmico, gratuito e de código aberto, feito
para fotógrafos. Toda ajuda conta: relatar um problema, sugerir uma ideia, testar uma versão nova
ou mandar código.

## Relatar um problema ou sugerir uma ideia

- **Sem conta no GitHub:** no Lamina, use **Ajuda → Enviar sugestão ou problema…**. O formulário já
  leva a versão do programa.
- **Com conta no GitHub:** abra uma [issue](https://github.com/alberthpalhares/lamina-app/issues/new/choose)
  e escolha o modelo (problema, sugestão ou problema na instalação).
- Se algo deu errado, **Ajuda → Abrir pasta de registros** mostra o `lamina.log`: anexe esse
  arquivo.
- **Nunca anexe fotos de clientes.** Se precisar mostrar uma lâmina, use o álbum de exemplo
  (tela de boas-vindas → Álbum de exemplo).

## Mandar código

Este repositório é o espelho público do código do Lamina; o desenvolvimento acontece num
repositório principal, com as especificações e os documentos do projeto. Pull requests aqui são
bem-vindos: depois da revisão, a mudança é levada ao repositório principal com o crédito de quem
fez, e volta para cá na próxima sincronização.

Antes de abrir o PR:

1. Rode `cargo xtask verify` (formatação, lint, testes e as checagens do projeto). O CI roda o
   mesmo comando.
2. Toda função pública nova chega com teste; bug corrigido chega com o teste que o reproduz. Os
   nomes dos testes descrevem o comportamento em português.
3. Nada de `unwrap`, `expect` ou `unsafe` fora dos testes.
4. Mensagens que chegam à janela são escritas em linguagem de fotógrafo, sem jargão.
5. Nenhum código, texto, imagem ou modelo de outro programa de diagramação.

Para mudanças grandes, abra antes uma issue para conversar sobre a ideia.

## Conduta

Todo mundo que participa segue o [código de conduta](CODE_OF_CONDUCT.md).
