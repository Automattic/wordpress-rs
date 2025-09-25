<?php
/**
 * Plugin Name: Books Manager
 * Description: A WordPress plugin that creates a custom post type for books with genre and author taxonomies
 * Version: 1.0.0
 * Author: Your Name
 */

// Prevent direct access
if (!defined('ABSPATH')) {
    exit;
}

class BooksPlugin {

    public function __construct() {
        add_action('init', array($this, 'register_post_type_and_taxonomies'));
        add_action('admin_menu', array($this, 'add_admin_menu'));
        add_action('admin_post_books_reset_data', array($this, 'handle_reset_data'));
        register_activation_hook(__FILE__, array($this, 'activate_plugin'));
    }

    public function register_post_type_and_taxonomies() {
        $this->register_books_post_type();
        $this->register_genre_taxonomy();
        $this->register_author_taxonomy();
    }

    private function register_books_post_type() {
        $labels = array(
            'name'                  => _x('Books', 'Post type general name', 'textdomain'),
            'singular_name'         => _x('Book', 'Post type singular name', 'textdomain'),
            'menu_name'             => _x('Books', 'Admin Menu text', 'textdomain'),
            'name_admin_bar'        => _x('Book', 'Add New on Toolbar', 'textdomain'),
            'add_new'               => __('Add New', 'textdomain'),
            'add_new_item'          => __('Add New Book', 'textdomain'),
            'new_item'              => __('New Book', 'textdomain'),
            'edit_item'             => __('Edit Book', 'textdomain'),
            'view_item'             => __('View Book', 'textdomain'),
            'all_items'             => __('All Books', 'textdomain'),
            'search_items'          => __('Search Books', 'textdomain'),
            'parent_item_colon'     => __('Parent Books:', 'textdomain'),
            'not_found'             => __('No books found.', 'textdomain'),
            'not_found_in_trash'    => __('No books found in Trash.', 'textdomain'),
            'featured_image'        => _x('Book Cover Image', 'Overrides the "Featured Image" phrase', 'textdomain'),
            'set_featured_image'    => _x('Set cover image', 'Overrides the "Set featured image" phrase', 'textdomain'),
            'remove_featured_image' => _x('Remove cover image', 'Overrides the "Remove featured image" phrase', 'textdomain'),
            'use_featured_image'    => _x('Use as cover image', 'Overrides the "Use as featured image" phrase', 'textdomain'),
            'archives'              => _x('Book archives', 'The post type archive label', 'textdomain'),
            'insert_into_item'      => _x('Insert into book', 'Overrides the "Insert into post"/"Insert into page" phrase', 'textdomain'),
            'uploaded_to_this_item' => _x('Uploaded to this book', 'Overrides the "Uploaded to this post"/"Uploaded to this page" phrase', 'textdomain'),
            'filter_items_list'     => _x('Filter books list', 'Screen reader text for the filter links', 'textdomain'),
            'items_list_navigation' => _x('Books list navigation', 'Screen reader text for the pagination', 'textdomain'),
            'items_list'            => _x('Books list', 'Screen reader text for the items list', 'textdomain'),
        );

        $args = array(
            'labels'             => $labels,
            'public'             => true,
            'publicly_queryable' => true,
            'show_ui'            => true,
            'show_in_menu'       => true,
            'query_var'          => true,
            'rewrite'            => array('slug' => 'books'),
            'capability_type'    => 'post',
            'has_archive'        => true,
            'hierarchical'       => false,
            'menu_position'      => null,
            'menu_icon'          => 'dashicons-book',
            'supports'           => array('title', 'editor', 'thumbnail', 'excerpt', 'comments', 'author', 'custom-fields'),
            'show_in_rest'       => true,
            'taxonomies'         => array('genre', 'book-author'),
        );

        register_post_type('books', $args);
    }

    private function register_genre_taxonomy() {
        $labels = array(
            'name'              => _x('Genres', 'taxonomy general name', 'textdomain'),
            'singular_name'     => _x('Genre', 'taxonomy singular name', 'textdomain'),
            'search_items'      => __('Search Genres', 'textdomain'),
            'all_items'         => __('All Genres', 'textdomain'),
            'parent_item'       => __('Parent Genre', 'textdomain'),
            'parent_item_colon' => __('Parent Genre:', 'textdomain'),
            'edit_item'         => __('Edit Genre', 'textdomain'),
            'update_item'       => __('Update Genre', 'textdomain'),
            'add_new_item'      => __('Add New Genre', 'textdomain'),
            'new_item_name'     => __('New Genre Name', 'textdomain'),
            'menu_name'         => __('Genres', 'textdomain'),
        );

        $args = array(
            'hierarchical'      => true,
            'labels'            => $labels,
            'show_ui'           => true,
            'show_admin_column' => true,
            'query_var'         => true,
            'rewrite'           => array('slug' => 'genre'),
            'show_in_rest'      => true,
        );

        register_taxonomy('genre', array('books'), $args);
    }

    private function register_author_taxonomy() {
        $labels = array(
            'name'                       => _x('Authors', 'taxonomy general name', 'textdomain'),
            'singular_name'              => _x('Author', 'taxonomy singular name', 'textdomain'),
            'search_items'               => __('Search Authors', 'textdomain'),
            'popular_items'              => __('Popular Authors', 'textdomain'),
            'all_items'                  => __('All Authors', 'textdomain'),
            'edit_item'                  => __('Edit Author', 'textdomain'),
            'update_item'                => __('Update Author', 'textdomain'),
            'add_new_item'               => __('Add New Author', 'textdomain'),
            'new_item_name'              => __('New Author Name', 'textdomain'),
            'separate_items_with_commas' => __('Separate authors with commas', 'textdomain'),
            'add_or_remove_items'        => __('Add or remove authors', 'textdomain'),
            'choose_from_most_used'      => __('Choose from the most used authors', 'textdomain'),
            'not_found'                  => __('No authors found.', 'textdomain'),
            'menu_name'                  => __('Authors', 'textdomain'),
        );

        $args = array(
            'hierarchical'      => false,
            'labels'            => $labels,
            'show_ui'           => true,
            'show_admin_column' => true,
            'query_var'         => true,
            'rewrite'           => array('slug' => 'book-author'),
            'show_in_rest'      => true,
        );

        register_taxonomy('book-author', array('books'), $args);
    }

    public function activate_plugin() {
        // Register post types and taxonomies first
        $this->register_post_type_and_taxonomies();

        // Flush rewrite rules
        flush_rewrite_rules();

        // Insert test data
        $this->insert_test_data();
    }

    private function insert_test_data() {
        // Check if test data already exists
        if (get_option('books_plugin_test_data_inserted')) {
            return;
        }

        // Create genres (hierarchical)
        $genres = $this->create_test_genres();

        // Create authors (non-hierarchical)
        $authors = $this->create_test_authors();

        // Create books
        $this->create_test_books($genres, $authors);

        // Mark test data as inserted
        update_option('books_plugin_test_data_inserted', true);
    }

    private function create_test_genres() {
        $genres = array();

        // Parent genres
        $fiction_term = wp_insert_term('Fiction', 'genre');
        $non_fiction_term = wp_insert_term('Non-Fiction', 'genre');

        if (!is_wp_error($fiction_term)) {
            $genres['fiction'] = $fiction_term['term_id'];

            // Fiction subgenres
            $mystery = wp_insert_term('Mystery', 'genre', array('parent' => $fiction_term['term_id']));
            $romance = wp_insert_term('Romance', 'genre', array('parent' => $fiction_term['term_id']));
            $sci_fi = wp_insert_term('Science Fiction', 'genre', array('parent' => $fiction_term['term_id']));
            $fantasy = wp_insert_term('Fantasy', 'genre', array('parent' => $fiction_term['term_id']));
            $thriller = wp_insert_term('Thriller', 'genre', array('parent' => $fiction_term['term_id']));

            if (!is_wp_error($mystery)) $genres['mystery'] = $mystery['term_id'];
            if (!is_wp_error($romance)) $genres['romance'] = $romance['term_id'];
            if (!is_wp_error($sci_fi)) $genres['sci_fi'] = $sci_fi['term_id'];
            if (!is_wp_error($fantasy)) $genres['fantasy'] = $fantasy['term_id'];
            if (!is_wp_error($thriller)) $genres['thriller'] = $thriller['term_id'];
        }

        if (!is_wp_error($non_fiction_term)) {
            $genres['non_fiction'] = $non_fiction_term['term_id'];

            // Non-fiction subgenres
            $biography = wp_insert_term('Biography', 'genre', array('parent' => $non_fiction_term['term_id']));
            $history = wp_insert_term('History', 'genre', array('parent' => $non_fiction_term['term_id']));
            $self_help = wp_insert_term('Self-Help', 'genre', array('parent' => $non_fiction_term['term_id']));
            $business = wp_insert_term('Business', 'genre', array('parent' => $non_fiction_term['term_id']));

            if (!is_wp_error($biography)) $genres['biography'] = $biography['term_id'];
            if (!is_wp_error($history)) $genres['history'] = $history['term_id'];
            if (!is_wp_error($self_help)) $genres['self_help'] = $self_help['term_id'];
            if (!is_wp_error($business)) $genres['business'] = $business['term_id'];
        }

        return $genres;
    }

    private function create_test_authors() {
        $authors = array();

        $author_names = array(
            'Stephen King', 'J.K. Rowling', 'Agatha Christie', 'George Orwell', 'Jane Austen',
            'Ernest Hemingway', 'Harper Lee', 'F. Scott Fitzgerald', 'Mark Twain', 'Charles Dickens',
            'Maya Angelou', 'Toni Morrison', 'Isaac Asimov', 'Ray Bradbury', 'Margaret Atwood',
            'Dan Brown', 'John Grisham', 'Nicholas Sparks', 'Gillian Flynn', 'Malcolm Gladwell'
        );

        foreach ($author_names as $author_name) {
            $author_term = wp_insert_term($author_name, 'book-author');
            if (!is_wp_error($author_term)) {
                $authors[] = $author_term['term_id'];
            }
        }

        return $authors;
    }

    private function create_test_books($genres, $authors) {
        $books_data = array(
            array(
                'title' => 'The Shining',
                'content' => 'A psychological horror novel about a writer who becomes caretaker of an isolated hotel.',
                'genre' => 'thriller',
                'author_index' => 0
            ),
            array(
                'title' => 'Harry Potter and the Philosopher\'s Stone',
                'content' => 'A young wizard discovers his magical heritage and attends Hogwarts School.',
                'genre' => 'fantasy',
                'author_index' => 1
            ),
            array(
                'title' => 'Murder on the Orient Express',
                'content' => 'Detective Hercule Poirot investigates a murder aboard a luxury train.',
                'genre' => 'mystery',
                'author_index' => 2
            ),
            array(
                'title' => '1984',
                'content' => 'A dystopian novel about totalitarian surveillance and thought control.',
                'genre' => 'sci_fi',
                'author_index' => 3
            ),
            array(
                'title' => 'Pride and Prejudice',
                'content' => 'A romantic novel about Elizabeth Bennet and Mr. Darcy in Georgian England.',
                'genre' => 'romance',
                'author_index' => 4
            ),
            array(
                'title' => 'The Old Man and the Sea',
                'content' => 'An aging fisherman struggles with a giant marlin in the Gulf Stream.',
                'genre' => 'fiction',
                'author_index' => 5
            ),
            array(
                'title' => 'To Kill a Mockingbird',
                'content' => 'A novel about racial injustice in the American South during the 1930s.',
                'genre' => 'fiction',
                'author_index' => 6
            ),
            array(
                'title' => 'The Great Gatsby',
                'content' => 'A critique of the American Dream set in the Jazz Age.',
                'genre' => 'fiction',
                'author_index' => 7
            ),
            array(
                'title' => 'Adventures of Huckleberry Finn',
                'content' => 'A young boy\'s journey down the Mississippi River with an escaped slave.',
                'genre' => 'fiction',
                'author_index' => 8
            ),
            array(
                'title' => 'A Tale of Two Cities',
                'content' => 'A historical novel set during the French Revolution.',
                'genre' => 'history',
                'author_index' => 9
            ),
            array(
                'title' => 'I Know Why the Caged Bird Sings',
                'content' => 'An autobiographical account of the author\'s childhood and youth.',
                'genre' => 'biography',
                'author_index' => 10
            ),
            array(
                'title' => 'Beloved',
                'content' => 'A novel about the lasting effects of slavery on individuals and families.',
                'genre' => 'fiction',
                'author_index' => 11
            ),
            array(
                'title' => 'Foundation',
                'content' => 'A science fiction novel about the fall of a galactic empire.',
                'genre' => 'sci_fi',
                'author_index' => 12
            ),
            array(
                'title' => 'Fahrenheit 451',
                'content' => 'A dystopian novel about a society where books are banned.',
                'genre' => 'sci_fi',
                'author_index' => 13
            ),
            array(
                'title' => 'The Handmaid\'s Tale',
                'content' => 'A dystopian novel about a theocratic society that subjugates women.',
                'genre' => 'sci_fi',
                'author_index' => 14
            ),
            array(
                'title' => 'The Da Vinci Code',
                'content' => 'A mystery thriller involving religious history and symbology.',
                'genre' => 'thriller',
                'author_index' => 15
            ),
            array(
                'title' => 'The Firm',
                'content' => 'A young lawyer discovers his law firm has dangerous connections.',
                'genre' => 'thriller',
                'author_index' => 16
            ),
            array(
                'title' => 'The Notebook',
                'content' => 'A romantic story about enduring love across decades.',
                'genre' => 'romance',
                'author_index' => 17
            ),
            array(
                'title' => 'Gone Girl',
                'content' => 'A psychological thriller about a marriage gone terribly wrong.',
                'genre' => 'thriller',
                'author_index' => 18
            ),
            array(
                'title' => 'Outliers',
                'content' => 'An exploration of what makes high-achievers different.',
                'genre' => 'self_help',
                'author_index' => 19
            ),
            array(
                'title' => 'The Tipping Point',
                'content' => 'How little things can make a big difference in social trends.',
                'genre' => 'business',
                'author_index' => 19
            ),
            array(
                'title' => 'Blink',
                'content' => 'The power of thinking without thinking - rapid cognition.',
                'genre' => 'self_help',
                'author_index' => 19
            ),
            array(
                'title' => 'It',
                'content' => 'A horror novel about a shape-shifting entity that preys on children.',
                'genre' => 'thriller',
                'author_index' => 0
            ),
            array(
                'title' => 'Harry Potter and the Chamber of Secrets',
                'content' => 'Harry\'s second year at Hogwarts brings new mysteries and dangers.',
                'genre' => 'fantasy',
                'author_index' => 1
            ),
            array(
                'title' => 'And Then There Were None',
                'content' => 'Ten strangers are invited to an island where they die one by one.',
                'genre' => 'mystery',
                'author_index' => 2
            )
        );

        foreach ($books_data as $book_data) {
            // Create the book post
            $post_id = wp_insert_post(array(
                'post_title'   => $book_data['title'],
                'post_content' => $book_data['content'],
                'post_status'  => 'publish',
                'post_type'    => 'books',
                'post_excerpt' => substr($book_data['content'], 0, 100) . '...'
            ));

            if ($post_id && !is_wp_error($post_id)) {
                // Assign genre
                if (isset($genres[$book_data['genre']])) {
                    wp_set_object_terms($post_id, array($genres[$book_data['genre']]), 'genre');
                }

                // Assign author
                if (isset($authors[$book_data['author_index']])) {
                    wp_set_object_terms($post_id, array($authors[$book_data['author_index']]), 'book-author');
                }
            }
        }
    }

    public function add_admin_menu() {
        add_submenu_page(
            'edit.php?post_type=books',
            'Books Settings',
            'Settings',
            'manage_options',
            'books-settings',
            array($this, 'settings_page')
        );
    }

    public function settings_page() {
        ?>
        <div class="wrap">
            <h1>Books Manager Settings</h1>

            <div class="card">
                <h2>Test Data Management</h2>
                <p>Use this button to delete all existing books, genres, and authors, then recreate the test data.</p>
                <p><strong>Warning:</strong> This will permanently delete all books and their taxonomies. This action cannot be undone.</p>

                <form method="post" action="<?php echo admin_url('admin-post.php'); ?>">
                    <?php wp_nonce_field('books_reset_data_action', 'books_reset_data_nonce'); ?>
                    <input type="hidden" name="action" value="books_reset_data">
                    <p>
                        <input type="submit" class="button button-secondary" value="Delete & Recreate Test Data"
                               onclick="return confirm('Are you sure you want to delete all books and recreate test data? This cannot be undone!');">
                    </p>
                </form>

                <?php
                $test_data_exists = get_option('books_plugin_test_data_inserted');
                if ($test_data_exists) {
                    echo '<p><span class="dashicons dashicons-yes-alt" style="color: green;"></span> Test data has been inserted.</p>';
                } else {
                    echo '<p><span class="dashicons dashicons-warning" style="color: orange;"></span> Test data has not been inserted yet.</p>';
                }
                ?>
            </div>

            <div class="card">
                <h2>Current Statistics</h2>
                <?php
                $books_count = wp_count_posts('books');
                $genres_count = wp_count_terms(array('taxonomy' => 'genre'));
                $authors_count = wp_count_terms(array('taxonomy' => 'book-author'));
                ?>
                <ul>
                    <li><strong>Books:</strong> <?php echo $books_count->publish; ?> published</li>
                    <li><strong>Genres:</strong> <?php echo $genres_count; ?> terms</li>
                    <li><strong>Authors:</strong> <?php echo $authors_count; ?> terms</li>
                </ul>
            </div>
        </div>
        <?php
    }

    public function handle_reset_data() {
        // Verify nonce for security
        if (!wp_verify_nonce($_POST['books_reset_data_nonce'], 'books_reset_data_action')) {
            wp_die('Security check failed');
        }

        // Check user permissions
        if (!current_user_can('manage_options')) {
            wp_die('Insufficient permissions');
        }

        // Delete all existing data
        $this->delete_all_test_data();

        // Recreate test data
        $this->insert_test_data();

        // Redirect back with success message
        $redirect_url = add_query_arg(
            array(
                'post_type' => 'books',
                'page' => 'books-settings',
                'message' => 'data_reset'
            ),
            admin_url('edit.php')
        );

        wp_redirect($redirect_url);
        exit;
    }

    private function delete_all_test_data() {
        // Delete all books
        $books = get_posts(array(
            'post_type' => 'books',
            'numberposts' => -1,
            'post_status' => 'any'
        ));

        foreach ($books as $book) {
            wp_delete_post($book->ID, true); // true = force delete, skip trash
        }

        // Delete all genre terms
        $genres = get_terms(array(
            'taxonomy' => 'genre',
            'hide_empty' => false
        ));

        foreach ($genres as $genre) {
            wp_delete_term($genre->term_id, 'genre');
        }

        // Delete all author terms
        $authors = get_terms(array(
            'taxonomy' => 'book-author',
            'hide_empty' => false
        ));

        foreach ($authors as $author) {
            wp_delete_term($author->term_id, 'book-author');
        }

        // Reset the option flag
        delete_option('books_plugin_test_data_inserted');
    }
}

// Initialize the plugin
new BooksPlugin();