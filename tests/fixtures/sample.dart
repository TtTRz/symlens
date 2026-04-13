import 'dart:async';
import 'package:flutter/widgets.dart' show Widget, BuildContext;

/// A base repository for data access.
abstract class Repository<T> {
  Future<T?> findById(String id);
  Future<List<T>> findAll();
  Future<void> save(T entity);
}

/// User model.
class User {
  final String id;
  final String name;
  final String email;

  /// Create a new user.
  const User({required this.id, required this.name, required this.email});

  /// Named constructor from JSON.
  factory User.fromJson(Map<String, dynamic> json) {
    return User(
      id: json['id'] as String,
      name: json['name'] as String,
      email: json['email'] as String,
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'name': name,
    'email': email,
  };
}

/// Mixin for logging capabilities.
mixin Logger {
  void log(String message) {
    print('[LOG] $message');
  }
}

/// User repository implementation.
class UserRepository extends Repository<User> with Logger {
  final List<User> _users = [];

  @override
  Future<User?> findById(String id) async {
    log('Finding user by id: $id');
    return _users.where((u) => u.id == id).firstOrNull;
  }

  @override
  Future<List<User>> findAll() async {
    log('Finding all users');
    return List.unmodifiable(_users);
  }

  @override
  Future<void> save(User entity) async {
    log('Saving user: ${entity.name}');
    _users.add(entity);
  }
}

/// Status of an operation.
enum OperationStatus {
  pending,
  inProgress,
  completed,
  failed,
}

/// Callback type for user events.
typedef UserCallback = void Function(User user);

/// Create a default user repository.
UserRepository createRepository() {
  return UserRepository();
}

/// Process a list of users.
List<String> processUsers(List<User> users) {
  return users.map((u) => u.name).toList();
}

const int maxRetries = 3;
