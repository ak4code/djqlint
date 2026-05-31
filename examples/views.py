from django.shortcuts import render

from .models import Book, Author


def dashboard(request):
    # P001: counting via len() pulls every row into memory.
    books = Book.objects.filter(published=True)
    total = len(books)

    # P001 on a direct manager chain + list().
    everything = list(Author.objects.all())

    # P002: truthiness test evaluates and caches the whole QuerySet.
    pending = Book.objects.filter(status="pending")
    if pending:
        notify_admins()

    # N001: related-field access inside a loop -> one query per book.
    for book in books:
        print(book.author.name)
        print(book.publisher.country.code)

    # Suppressed: developer has acknowledged this one.
    slow = len(Book.objects.all())  # noqa: djqlint-P001

    # Not flagged: exists() is the correct idiom.
    if Author.objects.filter(active=True).exists():
        pass

    # Not flagged: select_related means the relation is eager-loaded.
    for book in Book.objects.select_related("author"):
        print(book.author.name)

    return render(request, "dashboard.html", {"total": total, "all": everything, "slow": slow})
